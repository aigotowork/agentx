---
schema: scenario
description: Select and reuse tools for disposable VSCode host inspection without overstating automation or screen access.
tags: [vscode, tools, automation]
---

# Tools and reusable automation

Use this guide when choosing how to exercise a native surface, capturing visual evidence or replacing repeated exploratory commands with a maintained scenario. The feature owns its [goal, steps and expectations](../../scenarios/feature-scenarios.md); reusable mechanisms should remove repeated capture and packaging work while preserving that meaning. Read the active runner's source/usage before execution. A historical PID, port, window ID, profile or bridge file is not current resource authority.

## Select the interaction surface

| Needed observation/action | First adequate tool | Boundary |
| --- | --- | --- |
| Inspect contracts, contributions and source | `rg`, actual manifest/entry and installed VSCode type declarations | Static facts do not establish rendered availability |
| Create a dirty document/selection, observe active URI, invoke a bounded API or inspect registered commands | A disposable extension-host test/bridge using real VSCode APIs | API invocation does not establish discoverability or physical input |
| Inspect a Webview frontend and its Host message result | Select its actual owned frame/target and pair DOM/image observations with the extension-host request/result and independent effect | A standalone browser fixture or top-level Workbench DOM does not prove actual Code messaging, resource policy or rendering |
| Inspect/click a TreeView, QuickPick, Comments input, workbench action or capture its renderer | Owned Electron renderer through CDP when available | Top-level DOM/capture may omit native OS menus and Webview content |
| Inspect an OS context menu or actual composed window | OS capture restricted to the owned window ID | Capture availability, permission and native input are separate facts |
| Drive an OS menu that renderer input cannot reach | An already available, authorized native automation mechanism with verified target/focus | Do not promote an unvalidated key probe to a standard driver |
| Assert persistence, second-consumer integration or process exit | Actual filesystem/provider/CLI/process observations | Neither screenshots nor UI callbacks prove these effects alone |

Start with the existing adequate mechanism. Do not install an automation package, replace the extension architecture or build a general desktop framework to resolve one screenshot. Consider a shared tool only after an actual second consumer or repeated primitive justifies it.

If a concrete gap remains, consult [optional tool skills](scenarios.md#optional-tool-skills) for the researched Playwright and screenshot methods, prerequisites and adaptations. A downloaded SKILL or a research checkout does not establish tool availability.

## Collect and map checkpoint artifacts

Use [the acceptance checkpoint plan](acceptance.md#plan-the-three-layers-at-checkpoints) to select observations before acting. Keep the runner's retained facts and images as the source; attach their references to the owning scenario report. Reuse captured artifacts across unchanged checkpoints only when their scope/version still explains the claimed state. Never replace an unobserved runtime fact with a design value without labeling its origin.

| Needed checkpoint material | Current concrete source or tool choice | Retention and limit |
| --- | --- | --- |
| Semantic surface/layout | Extension manifest/public contributions as expected structure; actual renderer controls/labels, thread/resource relationships from Host facts; selected Webview frame or OS-menu capture | Retain the observed relationship and source reference. Do not infer visible/enabled state from contributions or claim unmounted virtualized rows are absent. |
| Document and application state | Consumer-owned bridge or public editor APIs for explicitly selected documents and relevant application facts | Retain URI/version/dirty/text, collection interval and omissions; a bridge is not a full runtime snapshot |
| Input focus and selection | Editor API observations plus actual renderer focus/active group | Active editor alone does not prove where typing goes; label API setup and renderer input separately |
| Rendered evidence | Renderer or selected Webview screenshot/DOM, or exact owned-window capture | Keep target, scale, interval and missing surfaces; separate reads are not atomic |
| Actions and effects | Actual scene calls, command observations, buffer/disk/provider readback and exits | Command settlement and image capture are not persistence evidence |
| Run and cleanup | Original runner manifest/results/lifetime records | Preserve original limits; historical handles do not authorize reconnect/replay |

Read the consumer's current runner usage and scene source before selecting a callable command. A smoke scene establishes only host/capture/lifetime; document editing, setup and menu paths each need their actual preconditions. Do not compose scene names into a presumed campaign or use a global last-run pointer. If an observation is absent, record the gap or extend the owned scene; never fabricate history or copy a business driver into guidance.

## Export a retained run

Use the installed saved-capture exporter's README, canonical schema and public CLI/API. Confirm its full validator/attachment/reader resources are present. The original application runner remains the only Code launch, action and lifecycle owner. An exporter reads explicitly selected saved artifacts into a new self-contained bundle; it cannot collect a missing runtime fact.

Supply a feature-owned scenario selection with steps, expectations, original action/check outcomes and layer descriptors, plus the exact saved run manifest and a new output directory. Keep the feature's independent effect checks in its scene; the exporter preserves their reported results without executing or judging them. Read the bundle through the linked canonical reader and compare the retained facts with the scenario expectation. Export success proves packaging, not a successful save or human acceptance.

For new live steps, bind the existing bridge/driver at the scene boundary and retain target/version and collection timing. Business actions, readiness, Webview frame/message binding and effect assertions remain consumer work. The prepared saved-export mechanism does not require rebuilding capture infrastructure, and it cannot fill a missing runtime observation. Historical exporter examples demonstrate only their retained inputs; a new host claim requires a new actual Code scene.

## Workbench CDP

A disposable runner can launch Code with `--remote-debugging-port=<owned-port>`, then use an available compatible WebSocket/CDP client. Check the selected Node runtime for required APIs; this observation does not change application engine constraints.

Bind the automation to its run manifest: owned process and port, unique window title and returned target ID/URL. Check the actual debugger listener is local; do not attach to an arbitrary first page or a user's existing debugging port. Keep the debugger lifetime inside the disposable host.

Useful operations are `Runtime.evaluate` for bounded DOM observations, `Input.dispatchMouseEvent` / `Input.dispatchKeyEvent` / `Input.insertText` for renderer input, and `Page.captureScreenshot` for the target renderer. Record direct DOM activation or fixture API edits if used. Their success is not proof of OS-level keyboard handling.

Use semantic selectors scoped to the active surface and a specific label. Match a QuickPick's label, not any substring in the whole row: a “Markdown” description may itself contain “YAML”. Reject ambiguous matches. Check visibility, clipping/viewport and enabled state before clicking; nonzero width alone is insufficient. For a coordinate click, verify the intended element at the point and wait for stable layout when the surface moves.

These are selection requirements, not guarantees supplied by every driver. A simple text/nonzero-width helper may lack [Playwright actionability checks](https://playwright.dev/docs/actionability). Use the actual scene observations to establish readiness and record unmet checks. Playwright's [Electron API](https://playwright.dev/docs/api/class-electron) is an experimental alternative to assess for a concrete gap, not proof that the current Code runner uses it or that native OS dialogs are automated.

Account for virtualized lists: DOM absence can mean the row is outside the rendered viewport. Use the actual search or bounded scrolling to expose it before concluding it is missing. When coordinates come from a screenshot, identify its CSS/device-pixel scale and crop origin; an OS window image and renderer input need not share coordinates.

Scope a Comments input to its intended editor group and thread. If the reply input is collapsed, expand its real placeholder before typing. Multiple open discussions or a long-draft editor make a global “first comment editor” selector unsafe.

Wait for an expected state such as a visible picker title, new native thread, saved record or child exit, with a bounded timeout. Fixed delays may allow animation to settle but do not prove readiness. On failure, save the current target, visible text/image and action log before deciding whether the click missed, the control was disabled or the product rejected the operation. Do not click Save again without checking the original effect.

Treat a Webview as another rendering boundary. Inspect available owned frame/target information or use an owned-window screenshot. Top-level `document.body.innerText` may contain only the tab title; it cannot establish that the Webview rendered its content.

## macOS window capture and permissions

On macOS, an existing owned capture helper can use `CGWindowListCopyWindowInfo` to obtain windows filtered by an explicit Code PID, then `/usr/sbin/screencapture -x -l <owned-window-id> <evidence-file>`. Keep helpers/output under the owned temporary or application test root. Revalidate the process/window mapping at capture time; do not copy example IDs from old evidence or capture the full desktop.

Renderer and native-window screenshots answer different questions. Native menus can appear in an owned-window image while the renderer query is empty. A presentation preference does not prove a menu moved into renderer DOM.

Window capture does not require sending OS input. If native input is necessary and authorized, establish the target process/window and current foreground immediately before the action. A command return value, `Page.bringToFront`, or an earlier focus check is not proof that the intended menu received a key. Check the resulting UI. Stop the probe if it modifies the source fixture or misses the target; restore only owned fixture data and retain the failure.

| Actual failure | Next bounded action |
| --- | --- |
| Renderer screenshot omits a native menu | Use an owned-window capture; keep the renderer result as a surface limitation |
| `screencapture` cannot create an image | Record exact stderr and current PID/window validity; distinguish a stale/inactive window from an observed permission denial before requesting access |
| AppleScript/System Events hangs | Bound and terminate only the owned probe; do not keep launching blocked copies or infer a permission cause without evidence |
| Swift helper fails SDK/module compilation | Record a toolchain failure; use an already adequate C/Objective-C/system alternative if bounded, without repairing the system SDK for this task |
| Key injection reports success but changes the wrong field | Stop native input, retain the outcome and use validated renderer actions or record the remaining gap |
| The OS explicitly denies required capture or native control | Continue independent checks. Report the denied API, required scene, available fallback and the precise permission needed by the actual controlling process |

Screen Recording and Accessibility/Automation concern different operations and controlling processes. Do not request all of them speculatively. Follow existing user/Task authority; a guide does not grant OS privacy changes. If a permission change is genuinely required, explain the actual denial and concrete action it would enable. Do not reset privacy databases, alter settings or treat elapsed time as permission.

## Reuse the current application harness

Use one documented launch → selected scenes → evidence → cleanup entry. The consumer owns the original extension activation, explicit per-run manifest, profile/root, process handles and native capture binding. Preserve its actual scene prerequisites and evidence limits. Reuse that mechanism before adding another launcher or reviving historical inline probes.

A captured native menu proves visibility; inspect recorded input before claiming invocation. Staged business scenes do not establish one uninterrupted unattended campaign. A helper that identifies a window does not thereby implement native keyboard input. Preserve unsuccessful probes as historical evidence, not default procedures.

## Turn exploration into maintained scenarios

When a launch, wait, menu operation, capture or cleanup sequence repeats, improve the owning runner/helper rather than adding another near-identical inline script. Keep fixture-specific assertions in scenes and generic mechanics in the driver. Preserve exploratory failures as evidence; promote only the useful procedure.

When maintaining or extending the runner, preserve these boundaries:

| Layer | Required behavior |
| --- | --- |
| Run owner | Pass an explicit per-run manifest to every driver/scene; own process handles, profile/root, port, evidence output and final cleanup |
| Driver | Bounded connection/readiness waits, exact surface/label selection, action log and capture; no business policy or implicit mutation retry |
| Scenario | Declare preconditions and required predecessor/fixture; assert UI plus independent effect; support the affected slice where it can run honestly |
| Evidence writer | Preserve input/output identity, result/limits and captured bytes; reject source/evidence path overlap |
| Finalizer | Close pending work, settle driver requests, observe owned exits and record cleanup on success, assertion failure, timeout and interruption |

Maintain one documented orchestration entry for launch → selected scenes → evidence → cleanup. Expose only useful options such as the supported executable/runtime, scenario selection and evidence destination. A selected scene must prepare its real prerequisites or reject an invalid starting state; it must not silently reuse another run's singleton pointer. These are maintenance requirements; inspect current code and evidence before treating each failure path or option as verified.

Keep executable helpers with the application until multiple actual hosts justify a shared tool. The guide owns the method; the runner owns current commands and side effects. After a consolidation, run the smallest representative real scene that verifies changed automation, then check its failure/cleanup path as needed. Guidance link checks alone do not prove runner execution.
