---
schema: task
status: completed
agent: space-uiux-kena
---

# Space UIUX design system and business-block implementation

## User outcome

User request #34293 authorizes a design-system specialist to establish the `web/agent-space` UIUX system and implement it in the real Space consumer. The result must provide one coherent foundation for tokens, themes, Components, Layouts and business Blocks, with UIUX scenes that operate the same React implementations consumed by Space. The work includes a systematic audit of existing rough interaction behavior; the conversation composer submit path and post-send draft clearing are the first explicit defect to resolve, along with any adjacent defects discovered through the representative flows.

This is a product UI and interaction implementation task, not a decorative component catalogue. Preserve the complete responsive Space, exact Agent identity, native/live data boundaries, Canvas/runtime/debug contracts, cat/mailbox work, existing running services and source provenance. Do not replace Canvas with a UIUX canvas or introduce a second design system.

## Professional owner and coordination

Controller: space-responsive-zonu. One fresh `design-system` specialist owns method application, UIUX resource/design/scene contracts, React implementation integration, interaction repairs and ordinary source fixes. No delegation. Existing scene-runtime owner is closed; coordinate shared scene files with its released boundary and with the active cat/mailbox owner before edits. Independent finite verification is assigned after implementation because UIUX resource validity, real consumer behavior and narrow interaction are separate evidence responsibilities.

Required method: `.agent-guides/local/roles/design-system.md`, `uiux docs design-system`, relevant UIUX component/structure/scenes/tokens/resources guidance, project interface requirements, and TypeScript/React contracts. The specialist must inspect actual installed UIUX types and commands before creating resources; do not invent manifest or scene formats.

## Structural result

Create or update a product-owned `web/agent-space/uiux/` collection (unless the specialist finds an existing canonical equivalent) with a concise system direction, foundation/semantic tokens, supported theme, resource declarations and executable scenes. Use the actual public React components in scenes. Establish reusable resource boundaries for the representative whole:

- shell/layout regions and responsive spatial contracts;
- accessible primitives and controls (button, icon button, disclosure, field, status/feedback, tooltip/fact);
- overlays/windows and focus/return behavior;
- conversation business Block: history, composer, attachments/context, pending/sending/error/empty states, draft retention and successful-send clearing;
- scene object anchor/controls and office reader Block where useful, reusing Canvas as the visual world;
- mailbox/related resource Blocks only where current consumer scope requires them.

The exact component split belongs to the specialist. Do not make a file per span or duplicate existing components solely to satisfy a catalogue. Business Blocks own cohesive task composition; product screens retain data queries, permissions, native effects and exact identity orchestration.

## Retained contracts and interaction invariants

- Same React component/resource source is consumed by Space and UIUX scenes; no mock-only implementation presented as production support.
- Token-backed semantic classes own authored styling; avoid scattered presentation literals and parallel theme sources.
- Composer is controlled and drafts remain page/selection-owned as currently specified. On a successful deliberate send, clear only the sent draft according to the actual receipt/result; on failure/unknown outcome retain input and show actionable feedback. IME/attachments/context and exact target fencing remain correct.
- Selection, camera, narrow layout, focus restoration, Escape/disclosure layers, reduced motion, long content, unavailable/empty/error states and native/live boundaries remain truthful.
- UIUX scenes are deterministic and safe: no native mutations, external network, credentials or automatic sends. Simulated actions/results are explicit and local.
- Canvas pixel world/runtime and `window.agentxSceneDebug` remain owned by scene runtime/debug adapters; React overlay components consume the existing frame/DOM contracts and do not create another clock.
- Existing cat/mailbox behavior remains intact; coordinate overlapping source ownership with @nigive/@mavema.

## Required working sequence and evidence

1. Recover representative user tasks and inspect current components/consumers and live composer/send contracts. Record current-to-target system model and decisions in this Task or linked UIUX design docs.
2. Verify actual UIUX CLI/resource/scene contracts (`uiux docs`, installed types), then establish the smallest coherent resource skeleton before filling implementation.
3. Implement canonical resources and scenes, wire actual Space consumer, and repair composer send/clear plus any material defects found in representative states. Preserve public behavior and update module guidance/README routes.
4. Run `uiux check web/agent-space` (or the exact supported project path) and the scoped Space type/build check once after coherent edits. UIUX check alone does not establish runtime behavior.
5. Hand off to an independent verifier for actual UIUX workbench/static output and real Space browser paths: representative desktop/narrow shell, conversation send success/failure/unknown, draft clearing/retention, focus/IME/attachments where relevant, overlay return, no Canvas/runtime regressions and scene loading. Inspect images only for explicit layout/hierarchy questions and record limits.

No root manifest/native/API/service changes, no :38023 restart, no package rollout or broad migration. Build outputs use fresh external paths and owned preview ports. Preserve unrelated dirty work. Source edits are limited to `web/agent-space/`, this Task and linked module guidance/docs, with coordination for files actively owned by cat/mailbox work.

## Completion boundary

Accepted source must include a discoverable UIUX system/resource/scene entry, actual component/Block/Layout consumption by Space, fixed successful-send draft semantics, UIUX structural validation, scoped type/build evidence, and independent runtime/browser evidence. Design-system maturity, full product redesign, all browser engines and live LAN adoption remain separate claims.

## Accepted local outcome

Controller accepts the completed Space UIUX system and real consumer integration using [independent verification](space-uiux-verification.task.md#final-independent-outcome-after-structural-repair-35570), reported complete in #35612. Canonical tokens/styles, Components, four Layouts and business Blocks are consumed by both Space and actual UIUX scenes through [the public entry and usage](../../web/agent-space/uiux/README.md). Conversation sends clear only the exact sent revision/attachment IDs on matching Succeeded; failure/unknown retain content, receipt lookup can complete after reader unmount, and newer edits are preserved without automatic resubmit.

Evidence establishes structural catalog validity, six context cases and seven actual LiveSpace adapter cases, all14 declared UIUX scene loads, final product/typecheck and installed UIUX builds, real product behavior with disposable native-shaped responses, overlay/host isolation, mailbox/exact-sender/history, Canvas/debug/anchor/single-clock/remount continuity and representative desktop/narrow/short rendering. The two material layout defects (brand overlap, then compressed primary composer) were repaired at the canonical owning layout. Final affected continuation records15 stable checkpoints across desktop,320/390 widths and short heights with chat→scene→chat; input and Send remain fully visible through every clipping ancestor before automatic scroll/click effects. Changed options/attachment disclosures, focus return, field retention and long history/context scrolling were actually operated.

Final product artifact: `/tmp/space-uiux-niro-final-20261115-r3`; generated static UIUX: `web/agent-space/uiux/dist/`. [Source/artifact binding](../../.bin/space-uiux-verification/source-artifact-binding.json), [layout](../../.bin/space-uiux-verification/short-layout.json), [disclosures](../../.bin/space-uiux-verification/disclosures.json) and [cleanup](../../.bin/space-uiux-verification/cleanup.json) retain exact evidence. Verifier inspected representative final images for hierarchy/readability/primary-action reachability; this is finite Linux local Chrome/light-theme acceptance. API/receipt supply was simulated, IME evidence synthetic; live native/LAN adoption, OS IME, other browsers, dark theme and exhaustive assistive-technology acceptance are not established.

All owned browsers/harnesses and preview ports18983–18987 are closed; obsolete verifier r1/r2 outputs removed. User services, installed binaries and native APIs unchanged. Implementation kena and independent verification niro close at their declared source/evidence boundaries; all source/resources released. No repeated checks or further source work is required here. Subsequent changes require affected evidence; rollout is a separate selected-target action with its own owner, not implied by source completion.

## Binding

Launch returned ready identity `kena` (batch `08119ea3`), local `design-system` role with explicit max reasoning override. Assignment explicitly accepted by `space-uiux-kena` in #34386; Task is active. Implementer owns professional design decisions and scoped checks; Controller owns integration and independent acceptance. Independent verifier niro completed its assigned evidence; Controller accepted the local outcome above.

## Implementation model and current work

Owner: @kena. Source candidate handed off in #35274; [independent verification](space-uiux-verification.task.md) is assigned to fresh design-system verifier niro. Overall local source acceptance is completed; see accepted outcome above. The representative flow is office selection → exact Agent anchor/roster → conversation/history/channel → controlled draft and attachments → deliberate send → receipt/recovery → local return. UIUX resources must own canonical presentation implementations, not simply export product-private code. Native requests remain in the product adapter; Canvas/runtime/debug remain unchanged.

| Layer | Owner and contract |
| --- | --- |
| Foundation/theme | Existing light Space palette/type/spacing promoted to system-owned CSS; Canvas art palette stays scene-owned. Do not promise an unimplemented dark theme. |
| Components | Native semantic controls, controlled Composer, facts and disclosure/overlay behavior. |
| Layouts | Office shell, navigation/content, chat split, independent conversation scroll regions. |
| Blocks | Exact conversation identity/channels/history/composer; scene object anchors/control toolbar with existing frame refs. |
| Product orchestration | App/controller selection, camera memory, exact drafts/context, native capability and receipts. |
| Scenes | Deterministic local fixtures consuming canonical components, with whole composition and success/failure/unknown/pending/narrow examples. |

Current evidence is intermediate only: two early UIUX structural checks and typechecks passed before the complete integration; the first draft extraction omitted message options, corrected immediately. These are not final verification. Tool friction: resource paths must stay inside their system; system-owned source and adjacent per-stem design/scenes are required. Initial re-export scaffolding is being replaced by actual resource ownership before handoff. Final checks will run only after coherent final edits; workbench/browser/runtime validation remains independently assigned.

Receipt semantics: only `Succeeded` establishes send completion. `Accepted`, `Running`, `Unknown`, failure or transport uncertainty retain the draft. A delayed receipt must clear only the original exact draft revision and sent attachment IDs, preserving edits and selection changes. Recovery must never resubmit automatically.

### Coherent source candidate

Canonical runtime entry: `web/agent-space/uiux/systems/space/space.web.ts`; styles: `styles/index.css` and authoritative light `styles/foundation.css`. Product startup and UIUX Provider use `SpaceTheme`. Canonical implementations cover the four Layouts, native Button/Feedback/Attachments/Composer, FactButton and shared overlay stack, ConversationBlock, ActorAnchor/SceneObjectLayer, SceneControls, ObjectWindow and HumanMailboxBlock. Product adapters retain native supply/effects, safe Markdown, geometry, exact identity, camera and existing long-press handlers. Cat owners directly released the shared files (#34864/#34948); no runtime/debug/cat sampler or native service source was changed.

Interaction repairs: (1) successful send now completes only on matching `Succeeded`, including delayed original receipt lookup; page-owned revision checks preserve newer edits, even edit-away/back to identical text; only sent attachment IDs are removed; (2) the first attachment can be added to an empty draft; (3) mixed image/text paste retains the text at the selection; (4) Composer has a synchronous duplicate-submit guard and native busy/disabled semantics; (5) failed/unknown outcomes remain near input; (6) model records show a read-only composer; (7) FactButton composes caller focus/pointer callbacks and descriptions rather than replacing them. Message intent/thread/reply options remain product-owned and present.

Resource scenes are local and explicit: complete Canvas-backed office/reader, actual Layouts, send success/failure/unknown/pending with manual receipt simulation, populated/empty/unavailable/error conversation, controls/facts, toolbar, window return, exact sender mailbox handling/history. Existing context tests gained authored revision/exact-target/attachment-only cleanup regressions; these have not been executed by this implementer. `@scope` protects UIUX chrome and container width rules follow actual Space width; modern-browser runtime and visual support remain an independent acceptance obligation. No preview/service has been started or modified.


### Implementation handoff and evidence

Implementation is released to @zonu for independently assigned UIUX/runtime/browser verification; this Task remains active pending that evidence, rather than claiming the full Completion boundary is accepted. Final coherent catalog check: `uiux check web/agent-space` exited 0 with 16 total resources (10 Space + 6 installed workbench) and valid `space`/`workbench` graphs. Product build (including scoped TypeScript) exited 0 at `/tmp/agent-space-uiux-kena-20261115-r1` with 96 modules, 92.90 kB CSS / 407.24 kB JS. A subsequent r2 build was launched after final host-boundary fixes; its PTY completion output was not retained, so no exit-status claim is made for r2. Final source `node web/agent-space/scripts/run.mjs typecheck` exited 0 after those fixes. The early skeleton checks and final build/type checks are explicitly separate; initial checks were premature and are not offered as final-source runtime evidence.

Final host-boundary corrections: Escape/shortcut disclosure queries are confined to each mounted Space interaction root, so scenes do not close UIUX shell disclosures. Fixed fact portals remain outside the sizing container while inheriting the same theme. Clipboard fallback appends its transfer input to the active Space theme boundary. Prior send feedback is suppressed in unrelated notes/model channels. These changed source guarantees justified the final typecheck; the valid catalog was not rerun.

No workbench dev/static build or browser operation was executed by the implementer, and authored new context regressions remain unexecuted. Independent verification must compile all UIUX scene/config modules, load every declared scene and actual symbol, operate representative complete desktop/narrow/short Space, run the new context cases, exercise actual send Succeeded/Failed/Unknown/Accepted-to-Succeeded recovery with exact revision/attachment fencing, empty-draft upload, mixed paste/IME, overlay focus/return and mailbox exact-sender/history behavior, and retain single-clock Canvas/debug/cat evidence. CSS scope/container changes especially require rendered comparison; compilation is not visual acceptance. No selected service, :38023, installed executable, root manifest/native/API source, preview process or rollout was changed.

### Independent narrow-header repair

Niro #35407 reports product/UIUX builds and all 14 declared scenes passing, plus six context and seven real LiveSpace receipt cases, real consumer send/recovery, Canvas/debug/remount/narrow checks. Its 320×492 screenshot (`.bin/space-uiux-verification/product-320-492.png`) exposes brand text overlapping the scene presentation label. Visual inspection confirms the brand flex item can shrink below its nowrap text width; the overflowing text covers the next button. Bounded canonical CSS repair: narrow `.office-hud` permits wrapping, and narrow `.office-brand` uses `flex: 1 0 auto` so the label retains its intrinsic width. Controls can move to another row while preserving labels, action access, font sizes and the same content-height/grid ownership. No TypeScript/manifests/runtime changes. Return only this CSS repair to niro for the affected 320/390 and short-screen rendered continuation; retain passed resource/receipt/runtime evidence and avoid duplicate implementer checks.

### Short-layout model correction (before implementation)

Niro #35516/#35524 confirms the header-wrap repair exposes a structural reader defect at 320×492: the footer shrinks to 55px while its form requires 145px, leaving 3px of Send visible. The failed invariant is ownership: the history stylesheet's more-specific `:has(.conversation-input)` footer rule permits shrink and retains an 80px history floor, while LaunchRelations separately retains non-shrinking context space. These competing allocations displace the primary task. A short-only composer minimum would preserve the conflict.

Selected correction: canonical ConversationLayout has an intrinsic identity row, bounded feedback row, one `minmax(0,1fr)` scrolling record/context body, and an intrinsic non-shrinking primary Composer row. Secondary context joins body scrolling; expanded message options/attachment previews use bounded local disclosure so content growth cannot push the input/Send pair out of the footer. The page still owns exact state, records, options and attachment effects. Remove the imported percentage/footer-shrink/history-floor declarations at their owner. Keep existing header wrapping, useful control sizes, history access and all native/runtime contracts. Independent continuation covers short/narrow input+Send default visibility plus secondary disclosure/return; unrelated passed evidence is retained.

The structural short-layout candidate is now released: `layouts/WorkspaceLayout.tsx` places secondary context inside record-body scrolling; `styles/conversation-layout.css` owns the four-row grid and local disclosure sizing; the imported shrink/48%/80px-floor declarations are removed from `history.css` and competing percentage slices from `workspace.css`. Core input and toolbar remain intrinsic. `ConversationComposer` exposes attachment-count disclosure and places the existing message-options slot in the toolbar; the product keeps all field values/effects and adds only the field-panel wrapper/accessible compact summary. Preview and fields are bounded above the toolbar by the actual `space-conversation` size container, so expansion does not push primary controls offscreen. Public props, native effects, exact refs and Canvas/runtime are unchanged. Source is ready for niro's single changed-layout build and affected desktop/320/390/short continuation, including attachments/options open/close. No implementer duplicate builds/checks or services.

### Independent finite verification returned

Niro #35612 reports the [independent verification Task](space-uiux-verification.task.md) COMPLETE/PASS for finite Linux local Chrome scope. This supersedes the earlier unexecuted/pending verification statements above for the exact final source/artifact binding recorded by that Task. Final r3 product/typecheck and UIUX builds exited 0; all 14 declared scenes loaded; six context and seven actual LiveSpace adapter cases passed. The actual consumer exercised success/retention/original-receipt recovery after reader unmount/no resubmit/newer revision/attachments/paste/synthetic IME, mailbox exact stopped-sender/history, Canvas/debug/anchor/three-remount/single-clock behavior, overlay focus and UIUX-host isolation. The affected-only continuation passed 15 stable desktop/320/390/short chat→scene→chat checkpoints with input/Send inside every clipping ancestor before scrolling, plus changed attachment/options disclosure Escape/focus/field retention and long history/context scrolling. Both material narrow findings are resolved in the final verified source.

Authoritative exact evidence and limits remain with the verifier Task and `.bin/space-uiux-verification/{short-layout,disclosures,source-artifact-binding,cleanup}.json`. Final product: `/tmp/space-uiux-niro-final-20261115-r3`; generated `uiux/dist` retained. Niro closed owned ports 18983–18987, browsers and harnesses. No implementer repeat checks were run. Source contribution and finite independent verification are complete; controller acceptance/Task closure remains @zonu-owned. This is simulated API/receipt supply and synthetic IME in Chrome, not live native/LAN adoption or all-engine acceptance; running services remain unchanged.
