---
schema: scenario
description: Accept VSCode extension behavior and visible usability through bounded disposable-host scenes.
tags: [vscode, acceptance, visual-review]
---

# Accept behavior in the actual Code host

Use when an extension change affects native navigation, input, configuration, target selection or process lifetime. Keep the canonical usage flow with the feature through [feature scenarios](../../scenarios/feature-scenarios.md), and link actual runs/results from its Task. Apply the shared [observable-feedback method](../../scenarios/observable-feedback.md); the sections below specialize it for Code. The guide supplies a method, not a mandatory full suite for every edit.

## Choose the evidence for the claim

| Claim | Evidence needed | Insufficient on its own |
| --- | --- | --- |
| A pure rule, DTO or state transition is correct | Focused test against its canonical contract | A screenshot |
| The actual extension loads and calls its provider correctly | Real Code activation, actual extension identity/entry, real provider and resulting state/file | Compilation or a replacement activation |
| A user can discover and invoke an action | Its visible surface and enabled state; actual menu/button/keyboard path for the claimed route | `executeCommand`, registered command list or manifest source |
| The operation affects the intended target | Original source/clicked resource plus the saved record or provider result | A success message or screen text alone |
| Information is readable and interaction preserves context | Inspect the captured image at its actual viewport; compare focus, source, draft and next action | Full DOM text hidden behind truncation |
| Shutdown releases owned resources | Exit observations for Code, extension host, admitted child/CLI processes and temporary-root cleanup | A resolved UI command or a timeout that killed only the parent |

Name controlled inputs precisely: source fixture edited through the real editor API, URI argument injected into a command, picker answer replaced, renderer mouse/keyboard input, or OS menu input. These establish different boundaries. Do not describe an injected URI as a clicked Explorer entry or automated interaction as a human usability study.

## Plan one coherent journey

Choose a small realistic material and the relevant boundary case. Record `starting state → visible action → resulting state/effect → evidence` before execution. Reuse existing storage/service evidence and test the host composition that changed.

Explain the user's goal and why each meaningful step matters before selecting a tool. Maintain the affected expectation and runnable binding when behavior changes; preserve the old basis with historical runs. For asynchronous save, the visible pending/success state explains the experience, original URI/version explains the target, and independent buffer/disk/provider readback establishes the effect. One cannot substitute for the others. A pure computation fix can stop at its focused contract evidence when it makes no new UI claim.

| Change or risk | Useful scene |
| --- | --- |
| First use and setup | Start without optional configuration; discover the entry, inspect readiness, cancel once, explicitly confirm setup and return to the same material |
| Comment authoring | Select actual dirty text and invoke the visible action. For separate feedback, compare the saved record and unchanged source disk. For inline Markdown, distinguish the buffer edit from the later explicit file Save. Inspect quote/body and continued source focus. |
| Long drafting | Expand, edit, return without saving, reopen the same draft and save; inspect body retention and ordinary Save behavior |
| Current reads and state | Use an actual second consumer to write a reply/change; return or refresh and inspect data separately from explicit status changes |
| Resource scope | Keep A open while invoking B's actual context menu; inspect the saved target and presentation without a fabricated source line |
| Configuration | Preview an actual next destination, cancel with unchanged bytes, save explicitly and verify the next operation uses that rule |
| Partial/unavailable data | Inspect a truncated list, malformed neighbor, unavailable resource or ambiguous quote; distinguish partial/error from empty and avoid a false location |
| Lifetime and concurrency | Switch scope with a retained draft, finish a late read, close during admitted work or while a native picker is unanswered |

Do not infer that all these scenes are required from the existence of this table. Select those required by the Task and substantive changed behavior. Test single-file/multi-root, theme, zoom or width variants when they can change the claimed result.

## Plan the three layers at checkpoints

For each meaningful checkpoint, plan and record semantic layout, necessary state and rendered evidence using the shared method. Group micro-actions when they reach one interpretable state, and reference unchanged retained artifacts. Give absent, partial, stale or inapplicable observations an explicit reason; do not silently omit a layer or require full process memory. Pure rules without a UI claim need only their focused contract checks.

| Layer | Code-specific observations | What to keep separate |
| --- | --- | --- |
| Semantic layout | Relevant views/editor groups, command/control IDs, roles, labels, thread/resource relationships and source/contribution references. Mark a manifest/layout design as expected; record actual visible controls separately. | A registered command is not a visible action. Workbench DOM, Webview structure and OS menus have different observation scopes. |
| State | Original target URI and scope, document version/dirty/selection, active editor and actual input focus, thread/draft identity and body, readiness/pending work, selected configuration and data observations needed to explain the scene. | Original source, active editor, draft URI and saved record are different identities. A bridge exposes selected facts, not all extension, Webview or external-process state. |
| Rendered evidence | Image and useful text/DOM/AX from the actual surface: Workbench renderer, selected Webview frame or owned OS window. Record viewport/window size, theme, zoom/scale and capture route. | A layout model or DOM dump is not an image. A renderer screenshot may omit an OS menu; top-level text may omit Webview content. |

Attach the action path, expected/observed comparison and independent effect to the same checkpoint references. Use run, scene/step and checkpoint identity plus original URI/version, renderer/frame/window identity and collection intervals where available. Before and after asynchronous capture, check whether the relevant target/version changed. Reject the combined-current interpretation when it did; retain the original samples and mark their limits. Do not infer atomicity from close timestamps or invent missing versions in older evidence. [Tool and artifact mapping](tools.md#collect-and-map-checkpoint-artifacts) identifies the current seams.

For an expanded inline comment draft, the following is a concrete checkpoint plan, not an assertion that a new run has occurred:

| Checkpoint | Semantic layout | Necessary state | Rendered evidence and independent comparison |
| --- | --- | --- | --- |
| Source selected | Source editor, selection action and intended thread location | Original URI/version/dirty/selection; source disk baseline | Source/selection image; confirm fixture API input is labeled |
| Draft expanded | Source/thread ↔ Markdown draft relationship and return/submit actions | Original target, active draft URI, body, focus, pending operations | Draft image; body readback agrees with expected input |
| Save cancelled | Actual simple Code dialog or OS dialog, named precisely | Cancelled operation settled; same retained body and target | Dialog/return image; source and export effects unchanged under the scene contract |
| Explicit submission and file Save | Return to original discussion and ordinary source Save | Inline buffer dirty after submit, then saved version/state | Source image plus separate buffer/disk observations; a draft export is not comment submission |

For a Webview, replace the native draft surfaces with the actual panel/frame and retain message request/resource identity, view generation, editor state and independent Host effect. For Explorer B while A stays open, keep both URIs and distinguish visible B-menu evidence from injected-URI handler evidence. Select only the changed branch, using the application's current scenes and their prerequisites.

## Run in a disposable host

1. Use the existing application runner after reading its current contract. Establish the exact Code executable/version, extension identity and build, external runtime, temporary workspace/profile/extensions directory and evidence destination.
2. Copy the real extension manifest and load its original activation/public entry. If a wrapper observes commands or process exits, record its changes and keep business behavior real. Do not add a test-only implementation of the feature.
3. Launch an isolated profile with explicit source/resource authority. Configuration that disables trust prompts, preselects a runtime or changes native presentation is a fixture choice; it does not prove first-use behavior. Keep setup scenes unconfigured when setup is the claim.
4. Drive the selected visible path with the [appropriate tool](tools.md#select-the-interaction-surface). Wait for its expected observable state before the next action. Never retry a write merely because automation failed to observe completion.
5. Collect the planned checkpoint layers and their independent data/process effects. Keep the expected/observed comparison and any missing or stale evidence visible. Keep failure evidence before correcting a selector or implementation. Record whether the defect was in the product, fixture, driver or environment.
6. Correct substantive defects at their owning design; rerun the affected scene only within the assigned integration or user-requested execution scope. Preserve earlier valid results and their build identity; a later run does not silently upgrade earlier evidence to a different source version.
7. Close owned work, observe process exits and remove the temporary workspace/profile only after evidence is retained. An unexplained live process or remaining resource is an explicit unfinished outcome.

Use short temporary roots where platform socket-path limits matter. Preserve installed extensions, user settings, real workspaces and immutable release evidence. Apply the Task's existing authority without adding approval steps for ordinary reversible fixture work.

## Inspect the pictures

Open screenshots only for UI design or review of how the interface serves user intent. State that question before reading and explain the visible hierarchy, content, actions, state and any mismatch afterward. For that purpose, use the actual screenshots with the available image viewer. Inspect at the recorded window dimensions, without using DOM text as a substitute for visual reading. For Webviews or OS menus, verify that the capture actually contains that surface; an empty renderer region is not proof of empty product content.

Ask the questions relevant to the scene:

- Can a user identify the next action, its enabled state and the object it affects?
- Are essential names, destinations, statuses and failure messages readable without reconstructing hidden text? Put decisive information before long contextual paths; an explicit copy/detail action can expose the rest.
- Does the source remain recognizable after opening Comments, saving, returning from a long draft or changing settings? Check focus, scroll, tab/group placement and whether a prompt or notification takes attention unnecessarily.
- Are loading, empty, partial, error and unknown-write states distinct? Does the interface preserve the body and offer an appropriate recovery action?
- Does the observed menu target agree with the resource saved? Does an unlocated or scope opinion avoid a misleading line anchor?

Treat lost drafts, wrong-target writes, duplicate/unknown replay, unreadable decisive information and unusable/missing actions as defects. Record noncritical cosmetic differences without starting another review cycle. Attribute native VSCode behavior separately from extension-triggered behavior; do not alter user preferences just to improve a test image.

## Retain checkpoints and follow-up feedback

Use the runner's existing facts, results and images as evidence sources. Add checkpoint references in the owning scene report; do not create a second editable result authority. Retain:

- The run ID, actual executable/runtime versions, original extension identity, relevant source/output hashes and controlled fixture settings.
- The scene/action path, expected and observed result, actual command/control IDs, substitutions and surface limits.
- Each checkpoint's three layer references or explicit missing/partial/stale reasons, collection intervals and resource/version correlation. Keep expected layouts distinct from observed structures.
- Screenshots plus useful text/DOM observations, independent saved bytes/results and errors. A screenshot title or filename is not an assertion.
- The owned PID/window/renderer mapping, process exits and cleanup result. Keep evidence output outside the source inputs and retained releases.
- Outstanding failures or unobserved behavior and the owner of any remaining acceptance step.

Use the installed saved-capture exporter's public entry to package explicit original run/step artifacts with feature-owned expectations. It creates a separate bundle with original bytes, digests, provenance and missing-layer reasons for the canonical offline reader. The selected adapter README supplies its public invocation and adaptation seam; the application runner retains live capture and lifecycle ownership. Preserve the original run and its result authority; an export and generated view are new readings of saved evidence.

Reference the same scenario, run/step artifacts and original comparison. The saved export neither reevaluates checks nor makes the consumer's Code runner emit a sidecar format natively. A new live capture is a new observation, not a repair of history.

Open saved artifacts to inspect history without attaching to Code, resolving old debugging ports or invoking old commands. Reconstructing a view requires explicit renderer/code/dependency/font/input prerequisites and produces a new result linked to the old node. Reexecuting a scene additionally requires a new owned host/fixture, current operation authority and original-effect recovery when a prior write is unknown. A saved state record cannot restore a process, request or external side effect.

For feedback, record the violated expectation, original checkpoint/artifacts, actual difference, effect status and proposed next action in the existing task or authorized feedback surface. Keep discussion separate from fixed evidence. After a correction, run the affected scene with a new identity, compare the same relevant expectations and link it back. A new passing test does not change the old observation or automatically establish human acceptance.

Finish when the required scenes have the evidence they need and substantive fixes are verified. If a required surface is inaccessible, report the exact missing claim and tool/environment failure to its owner. Continue independent checks, but do not relabel command wiring as visible acceptance.
