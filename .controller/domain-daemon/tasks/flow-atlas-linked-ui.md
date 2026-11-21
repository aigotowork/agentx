---
schema: task
status: completed
owner: paseo-3ab534dd-f30d-48f4-a0cb-30bdc2ee0a77
description: Design and implement a linked collaboration and scenario sequence workspace using existing semantic facts and one presentation clock.
---

# Linked collaboration workspace: UI assignment

## Accepted implementation and closure

Controller completed independent integration, returned bounded visual findings to this original owner, accepted the released derivatives and published the exact tested bytes. [Parent final outcome](flow-atlas-linked-workspace.md#accepted-linked-workspace-and-publication) and [final handoff](/tmp/flow-atlas-linked-gbdtzh0f/final-handoff.json) own10 authored groups,7 affected derivative groups,2 final terminal groups, LAN-origin behavior and preservation. All source claims are released; no pending source defect or required execution remains. Source authorship and controller execution remain distinct. Controller completed the docs-only current-support receiving in workbench/experience/tool entries after publication; executable inputs stayed frozen. Earlier pending language below is historical Task chronology. Human preference acceptance remains open.

Runtime binding: `{serverId: srv_yDbm9zuqIMiM, agentId: 3ab534dd-f30d-48f4-a0cb-30bdc2ee0a77}`; selected profile default, codex/gpt-6.1-sol, full-access/max. Controller owns independent acceptance.

Read [parent implementation/acceptance Task](flow-atlas-linked-workspace.md), [experience continuity design](../../../tools/flow-atlas/design/experience.md), native [UI](../../../tools/flow-atlas/workbench/AGENTS.md), [domain contract](../../../tools/flow-atlas/domain/contracts.ts), [experience](../../../tools/flow-atlas/prototypes/experience/AGENTS.md), and the [orchestration contract](../orchestration.md). The user's newest instruction authorizes implementation; historical source freezes apply only to old receiving assignments and do not bar this scoped work.

## Professional authority and method

Own cohesive UX/interaction design and its implementation. Apply the local/shared interface standard and management-product workflow: select the question, model the resources, design layout first, materialize a skeleton/explicit contracts, fill behavior, retain useful design decisions. Follow shared implementation/communication guidance. No Design Canvas plugin is available; use established HTML/SVG and tokens. No dependencies or additional delegation. The controller independently executes acceptance; do not run Node/Python/build/browser/tests/formatters/product/services yourself.

## Scope and constraints

Own presentation sources under tools/flow-atlas/workbench/**, necessary experience composition leaves under prototypes/experience/**, design/experience.md and your original Task. You may add cohesive pane/projector modules and register them in builders; avoid copying the whole app into another prototype. Domain/core/spatial/layout engine semantics and old generated artifacts are read-only. Controller owns tests, generated outputs, new server and routing in tool/root AGENTS/README. Keep prior single-view capabilities and generic consumer functioning. Preserve all native safe DOM/source semantics and hooks where possible. Do not overwrite old HTML/receipts or operate any runtime.

Deliver a useful default paired workspace (collaboration flow | scenario sequence), with single/paired modes and a compact auxiliary choice if useful. Main view combines object responsibility lanes, operation steps and conditional flows; select a step to reveal parameter/owner state evidence using existing authoritative facts. Design-only sequence must not infer execution from declaration order. One player/context/semantic selection serves both panes, each with its own viewport/camera, no iframe/second app/second clock/duplicate DOM IDs. Bidirectional selection must preserve panes and exact occurrence references; repeated visits cannot be conflated. Keep complete branch design distinct from the scenario's visited path and from the currently applied prefix. No invented reply for local-outcome and no unknown=success.

Desktop must show both useful diagrams legibly; inspector should disclose locally without squeezing both surfaces into unreadable thumbnails. Narrow screen uses primary plus accessible secondary disclosure/tab without losing context or creating tall document sections. Shared scenario/player must remain within the workspace. Existing checkpoint rail and caller/effect evidence remain useful, not duplicated everywhere. Different-scenario synchronized playback is not required; accurately retain current comparison boundaries.

## Early contract and release

Before filling implementation, publish structural design/pane contracts and stable hooks in this Task and send the controller an actionable handoff. Suggested hooks (you may refine explicitly early): linked-mode, linked-secondary-view, linked-pane-primary, linked-pane-secondary, linked-focus; pane roots include data-pane, data-view, data-scenario-id, data-checkpoint-index; semantic occurrence marks include data-step-id/data-visit-id/data-call-id. Expose detached pane camera/focus snapshots through FlowAtlasWorkbench.state() for independent acceptance without writable runtime access. Keep selection from a pane from forcing view replacement.

Proceed autonomously after publishing the skeleton; no approval round is needed. At coherent source release name exact builder command, scope, hooks, expected edge behavior and known limits; release all source claims. Update current detailed design, workbench/experience README and AGENTS for future maintainers, clearly distinguishing authored cases from controller execution. The parent Task owns final publication/current support. Repair only concrete substantive findings.

## Structural contract (2026-11-11, source author)

Question: investigate one process and one scenario, correlate a declared conditional collaboration with its exact recorded calls, then inspect parameters and owner state while retaining the applied checkpoint. Native `domain/contracts.ts` owns every fact; `app.js` owns one context/selection/player. `linked.js` owns presentation composition and independent pane cameras; `collaboration.js` projects responsibility lanes, operation cards, declared flows and occurrence chips. `views.js` continues owning the other projections and occurrence-aware sequence animation. Both builders register the same leaves. No second app, artifact reducer, player, dependency or iframe.

```text
CanvasWorkspace
├─ ExistingViewStrip (explicit primary view choice)
├─ LinkedToolbar (mode · auxiliary view · narrow active pane · semantic focus)
└─ PaneLayout
   ├─ PrimaryPane (collaboration default; existing canvas and layout controls)
   └─ SecondaryPane (scenario sequence default; optional state; own camera controls)
SubjectInspector (local overlay, never a third desktop column)
ExistingScenarioDock (one player, rail and caller/effect evidence pair)
```

| Surface | Stable contract |
| --- | --- |
| `data-testid="linked-mode"` | Select `paired` (default) or `single`; primary view is preserved. |
| `linked-secondary-view` | Select `sequence` (default) or `state`; changing it never changes process, scenario, checkpoint or semantic selection. |
| `linked-pane-primary`, `linked-pane-secondary` | Pane roots with `data-pane`, `data-view`, `data-scenario-id`, `data-checkpoint-index`; exactly one original `#canvas`. |
| `linked-show-primary`, `linked-show-secondary` | Narrow-screen tabs; same workspace region, cameras and focus retained. Desktop shows both in paired mode. |
| `linked-focus` | Compact selected step/occurrence label; semantic identity also in detached state. |
| Semantic marks | `data-step-id`; exact occurrences additionally have `data-visit-id`, `data-call-id`, `data-scenario-id`. References retain `kind/id/scope`, supplemented by `processId/stepId/scenarioId/visitId/callId/phase` when available. Definition clicks never silently choose among repeated visits. |
| `FlowAtlasWorkbench.state().linked` | Detached `{mode, secondaryView, activePane, focus, panes:{primary,secondary}}`; focus is `{processId,stepId?,scenarioId?,visitId?,callId?,phase?}` or null. Each pane is `{view,visible,camera,focus,scenarioId,checkpointIndex}`. `camera` is detached `{x,y,zoom}` or null. No writable runtime API. |

Interaction decisions: pane-origin selection opens details and reveals matching marks without replacing either pane or seeking the player. Explicit view controls alone change projection. All declared flows remain visible; scenario path and applied prefix have separate attributes/styles. In design-only sequence, show a scenario chooser prompt and no chronological calls. Sequence maps requests/replies/animation by exact call/visit, including repeated steps; no return arrow without `completion === 'reply'`. Future occurrence details use that visit's authored input plus applied evidence only, never a future checkpoint as current fact. Inspector overlays and closes back to the originating pane; narrow mode replaces the visible pane locally, not by stacking a second page section.

Method record: applied the management-product task/resource/layout method and shared/local interface standard. The existing canonical model is sufficient; observed source-level friction is view replacement in `app.select` and step-keyed sequence paths. Rejected a second embedded viewer/clock and CSS-only overlay because neither solves exact identity/context ownership. Desktop legibility and narrow reachability remain controller-executed acceptance, not source-author claims.

## Coherent source release (2026-11-11)

Source implementation and maintenance routing are complete and all source claims are released to the controller. This is source completion only; the [parent Task](flow-atlas-linked-workspace.md) owns independent execution, substantive findings, generated bytes, publication and actual acceptance. No Node/Python/build/browser/tests/formatter/product/service execution, dependency changes, further delegation, generated-artifact writes or runtime operations were performed by this source author.

| Sources | Released responsibility |
| --- | --- |
| `workbench/{collaboration.js,linked.js,linked.css}` | Compact responsibility-lane design, complete branches, exact occurrence chips; paired/single composition, independent cameras, local focus, responsive pane tabs and inspector treatment. |
| `workbench/{app.js,dom.js,views.js,details.js,build.py}` | One canonical context/selection/player; occurrence-preserving references and details; scenario-only sequence, exact call-keyed animation and namespaced markers; JSON-only builder registration. |
| `prototypes/experience/{build.py,projection.js,enhancement.js}` | Shared modules embedded in the existing shell; applied-occurrence parameter lens; exact occurrence history and stable view-button-only events. |
| Workbench/experience `AGENTS.md`, `README.md`; `design/experience.md`; this Task | Current contracts, design decisions, usage, authored boundaries and original published-evidence separation. |

From repository root, controller-owned generation can use `python3 tools/flow-atlas/prototypes/experience/build.py tools/flow-atlas/examples/domain-workbench.json --output /tmp/flow-atlas-linked-order.html` and the same command with `domain-workbench-technical.json` / `/tmp/flow-atlas-linked-technical.html`. The generic native composition is `python3 tools/flow-atlas/workbench/build.py tools/flow-atlas/examples/domain-workbench.json --output /tmp/flow-atlas-linked-native.html`. All outputs are new paths; no old output or server is replaced. Both builder receipts include the new presentation leaves.

Supplement to the early hook contract: request/reply/effect/local-result marks use `data-message-kind="request|reply|effect|local-outcome"`. Flow paths use `data-flow-id`, `data-scenario-path="true|false"` and `data-applied="true|false"`; occurrence marks use the same membership/prefix fields plus exact step/visit/call/scenario identity. `data-selected` exposes semantic focus. Sequence is initially `data-requires-scenario="true"` in design mode. `linked-occurrence-evidence` exposes the local selected-call/parameter/owner-state region. Camera scope is per pane/view/process/scenario; hidden panes return `camera:null` if the current scope has not mounted, while prior per-scope cameras remain retained. Narrow-screen tablist supports arrows/Home/End, ordinary controls remain keyboard reachable, and Escape/close returns to local origin.

Expected edge behavior for the independent cases: all declarations remain in collaboration; only scenario visits appear in sequence; future input is explicitly labelled and no future checkpoint is treated as applied. A repeated step's plain definition click has no visit/call, and each chip/message selects its own occurrence. Each applied parameter lookup is constrained to both step and visit within the applied prefix. Request/reply paths and their illustrative marker are keyed to call identity; local-outcome never generates a return. Selection, pane switching and camera movement do not reset player elapsed; context changes cancel the old player and clear old semantic focus. One `WorkbenchPlayer.create` remains in `app.js`; additional projections introduce no timing source. Parsed artifact objects are read-only throughout.

Known limits: desktop uses a fixed1.35:1 split rather than a draggable divider; long/large graphs remain readable through independent pan/zoom and may require navigation. Collaboration edge layout is a bounded projection, not general obstacle routing. Different-scenario comparison stays final snapshots, not synchronized playback. Repeated visits are presentation identity support, not a new compiler-loop or arbitrary-execution promise. Rendered density, clipping, keyboard/touch operability and all execution results remain unverified by this source author and must be assessed by the controller's affected L1–L9 cases/images. Only concrete substantive findings should reopen these released sources.

## Bounded visual findings and derivative

Controller reopened affected presentation sources after its independent first pass. Source author inspected controller images `/tmp/flow-atlas-linked-gbdtzh0f/browser-1/05-paused-transition.png`, `07-mobile-sequence.png`, `08-mobile-collaboration.png` and `11-native-composition.png` against three questions: can the operation and transport remain readable while input moves; does changing the visible pane reveal the retained exact interaction; does each responsibility/state label remain inside its own lane? These are direct image observations, not author-executed browser checks.

| Finding and cause | Bounded design/repair |
| --- | --- |
| Paused request payload overprints the operation caption and primary branch label. The unbounded moving text uses the same offset as static labels. | Keep the moving transport dot; disclose the current illustrative payload through a compact, bounded pane-heading cue/popover. Full payload remains safely rendered as text, scoped to the same call/visit/phase, and never overlays operation/branch names. |
| Returning from mobile sequence retains reserve identity but primary reserve remains offscreen. Reveal runs only when semantic focus changes, while hidden→visible preserves the same focus key. | Detect a pane newly becoming visible after mounting, then reveal its retained exact occurrence with that pane's existing camera. Preserve zoom, camera scope and presentation elapsed; do not seek or fit/reset. |
| Responsibility text crosses lane boundaries in desktop/native and mobile views. Character-count shortening underestimates CJK glyph width; no paint bound exists. | Use width-aware visual shortening and a namespaced per-lane SVG clip bound for identity/responsibility/state text. Keep ordinary font size, full accessible header/title and existing inspector content. |

Only `workbench/{views.js,linked.js,linked.css,collaboration.js}` and the corresponding design/maintenance notes are reopened. Controller retains all unaffected behavioral/native/clock/layout evidence and owns affected-only generation/paused/mobile/visual reruns. No source-author execution or resource changes are authorized or performed.

### Derivative source release

The bounded derivative is authored and all reopened source claims are released. Four executable leaves changed: `views.js` retains the precise transport dot and puts its full payload in an inert SVG title rather than unbounded moving text; `linked.js` owns the compact heading disclosure and reveal-on-visible transition; `linked.css` bounds the payload popover without shrinking the diagrams; `collaboration.js` bounds header glyphs and clips their painting to a namespaced lane region. `design/experience.md` and workbench `README.md`/`AGENTS.md` retain the reasons and maintenance ownership. No builder, artifact, domain/native contract, player, shared camera/layout engine or experience wrapper executable source changed in this derivative.

Hook refinement: each pane heading has `data-testid="linked-payload-cue"`, carrying exact `data-call-id/data-visit-id/data-phase` during illustrative request/reply motion. Its keyboard/touch-accessible summary discloses a bounded full-text `<pre data-testid="flow-payload">`; this preserves the hook name but intentionally changes its element/region from moving SVG text. `flow-marker` remains the exact animated SVG dot. Payload is illustrative target input/reply data, with its existing reply-completion guard; it never becomes applied evidence. `data-lane-header` identifies each clipped responsibility header. Clip IDs derive from the projection's namespaced marker ID plus lane index; full text is retained in inert titles and the accessible reference name.

Affected-only controller cases: pause request/reply and verify operation/branch captions remain readable while both pane cues expose the correct full payload; open/close the cue with keyboard/touch/Escape; select reserve in mobile sequence, close details and return to collaboration, verifying that exact occurrence is visible and zoom/scope/elapsed/checkpoint remain intact; inspect desktop/native/mobile header responsibility and state bounds, full accessible content and unique SVG IDs. All unchanged first-pass behavior/native/clock/layout evidence remains with the controller; source authors did not rerun or claim it. Rebuild commands remain exactly those in the first source release, using new derivative output paths.

### Terminal-label derivative source release

Controller reported all seven prior affected groups passing and reopened one concrete technical-model label collision. Source author inspected `/tmp/flow-atlas-linked-gbdtzh0f/browser-final/09-technical.png`: the188px terminal card places `Layout may move` and `completed` on baseline25, causing the semantic outcome to merge into the title. This repeats the text-width ownership issue rather than changing model or timing behavior.

The only executable change is in `workbench/collaboration.js`: end-step title and outcome use separate baselines18/36 within the unchanged44px height, retain ordinary `domain-title`/`domain-muted` fonts, and reuse width-budgeted text with full inert titles. Start cards, operation cards, branches, all IDs/hooks, clocks, cameras and other UI sources are unchanged. `design/experience.md` records the layout decision. This minimal derivative is source-complete and all source claims are released; controller owns only the affected technical/definition build/image checks and subsequent publication. No source-author execution or runtime/artifact writes occurred, and all prior passed groups remain controller evidence.
