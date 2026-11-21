---
status: completed
scope: Management web console source and isolated browser preview
owner: Management web
---

# Management console redesign

Start with the [Management scenario model](../../.docs/web-product-design/management.md) and [technical ledger direction](../../.docs/web-product-design/management.style.design.md). This task implements their MG-01 through MG-04 paths in the actual Management entry. Native Service and the public SDK remain authoritative; browser fixtures do not establish live adoption.

**Craft Read:** operator console for maintainers of one daemon dataset, product language, cool ledger light/dark with established cobalt, variance 4, signature bet: a precise selection rail and an operation receipt that appears where the action began.

| Entry | Current friction | Target shape and ownership |
| --- | --- | --- |
| Agents | Office catalog is nested inside Agent detail; repetitive identity metadata competes with conversation | Resources owns roster, filters and exact identity/actions; Conversation owns messages and transcript. Detail, drafts and filters survive destination changes. |
| Workspaces | Empty workspace and routing start from someone else's Agent | Offices owns independent known workspace index, effective roles, loaded members and advisory directory resolution. Choosing a workspace is independent from filtering Agents; a member links to its exact Agent. |
| Operations | A drawer competes with primary work; multiple long receipts are presented equally | Operations owns attention/history index and selected receipt; origin retains a concise status and a link to the exact receipt. |
| Shared | Connection context disappears at narrow sizes; controls are visually undifferentiated | Navigation owns persistent scope and hash destinations; Presentation supplies icon, status and empty primitives. CSS owns semantic tokens and responsive layout. Launch and stop use focused dialogs. |

P0: dataset and navigation, selected exact subject, available next action, scope/coverage, uncertainty. P1: label edits, provenance, transcript, route disclosure, original operation payload. Phone: index/detail switch for Agents, fitted destination bar, independently scrolling content and visible composer. No speculative metrics: the API exposes bounded records. No name-only navigation from launch receipts. Directory preview remains advisory; writes are recorded before submission.

Tool setup friction: ui-craft 1.0.21 had no installed Codex skills and its noninteractive installer writes harness configuration globally; project-local extraction in a disposable directory supplied the skills and references while preserving active harness settings. No connected ui-craft MCP was available; the design uses the skill's on-disk brief/tokens/redesign/dashboard guidance and the CLI detector.

## Delivered behavior and evidence

The actual Management entry now supports the three destinations, keyboard search and send, responsive selection and composing, followable local receipts, and history navigation. Directory contact requires a preview for the current path, and the result links to the exact returned Agent. An embedded origin retains compatible local data; separately selected listeners use distinct local namespaces. Generation changes fence retained Agent/workspace selections and operation lookup/replay. Confirmed message operations clear only their unchanged draft; an uncertain matching draft remains available with an original-request link. Management still consumes the current generated SDK; production and installed AgentX resources were not changed.

| Check | Evidence |
| --- | --- |
| Type and asset build | Strict Management typecheck and Vite build emitted the finite native-compatible manifest and hashed assets in the disposable final build. |
| Browser scenarios | Management and labels fixtures exercise independent workspace entry, empty workspaces, loaded-only filters, exact live/stopped references and drafts, keyboard search/send, stop confirmation and resume, model snapshots, Role launch, directory preview/result, same-ID lost-reply recovery, Unknown, listener and generation fencing, label conflict/rebase, and phone-sized reachability. Older fixtures needed the current generated Agent view/status and error-code fields. |
| Render and accessibility | [Desktop Agents](../../.artifacts/management-ui-craft/agents.png), [Workspaces](../../.artifacts/management-ui-craft/workspaces.png), [Operations](../../.artifacts/management-ui-craft/operations.png), [Launch](../../.artifacts/management-ui-craft/launch.png), [phone](../../.artifacts/management-ui-craft/mobile.png) and [phone in dark mode](../../.artifacts/management-ui-craft/mobile-dark.png). Chrome captured 1440×900 and 390×844 surfaces; the [axe report](../../.artifacts/management-ui-craft/accessibility.json) records no reported WCAG A/AA violations or horizontal document overflow in those captures. This is automated coverage, not assistive technology or physical-device acceptance. |
| UI Craft detector | [Final scan](../../.artifacts/management-ui-craft/ui-craft-detect.json): zero critical findings, one warning, matching the baseline counts. The warning is on the SDK adapter, where the UI states belong to the consumer. |
| Preview updates | [Manual refresh probe](../../.artifacts/management-ui-craft/preview-refresh.json) confirms the open page and draft stayed stable while the build changed and normal reload loaded the new assets with the draft intact. The [module guide](../../web/management/AGENTS.md) owns the repeatable preview command. |

The existing native Office and Labels browser drivers were adjusted to follow the new destinations and passed syntax checks. They were not rerun against a fresh Rust executable; no live-service rollout, real Provider, or human usability acceptance is claimed.

## Interaction refinement

The operator found the first implementation visually coarse and asked for much finer, modern, elegant interaction with particular attention to dialogs and generous, purposeful motion. **Craft Read:** operator console, product language, cool ledger and cobalt, variance 5, craft 9, motion 7; the same interaction language carries intent from selection to confirmation to the original receipt. The implementation uses ui-craft Animate, Polish, Delight, Forms, Modals and AI Chat references. The first build remains baseline evidence above.

| Direction | Decision |
| --- | --- |
| Surface refresh only | Leaves focus, dismissal, reply context and outcome feedback inconsistent. |
| Exhibition motion | Scroll and entrance choreography would slow repeat work and distract from exact execution/receipt state. |
| Precision workspace | Selected: shared modal choreography and focus contract; composed launch and stop decisions; a readable message stream, anchored replies, reliable new-message handling, and feedback at real state transitions. Quiet shell motion and a richer surface system unify Workspaces and Operations with the same controls. |

**Structure:** `dialog.tsx` owns native modality, dismissal and focus; Launch and Agent Toolbar own their independent decisions. `conversation.tsx` owns chronological grouping, reply and reading continuity. `operations.tsx` retains receipt authority while giving a real background state transition a visual marker. `styles/interaction.css` owns animation timing and presentation refinements; the existing storage, SDK and exact identity contracts remain authoritative. CSS transitions use discrete top-layer exit where available and serve a fully operable reduced-motion rendering. No animation gates submission or delays data reads.

| Before | After | Why |
| --- | --- | --- |
| Launch shows a flat field list; stop is a short generic confirmation | Both share a controlled modal with a consistent heading, scrollable body, fixed actions, Escape/backdrop dismissal and focus return; launch distinguishes source, location and task, and stop shows the exact execution | The operator sees what will happen before committing, and can recover unfinished input after interruption. |
| Replies expose identifiers through options and the user must find the referenced message | A quote links to the original; composing a reply shows the excerpt and offers one-step removal; new messages respect the reading position | Conversation context is visible without exposing an advanced field by default. |
| Operation feedback changes mostly as text | Real receipt transitions change shape and tone; shared selection, press and focus transitions respond immediately | Visual feedback follows the real outcome; the selected execution and original receipt remain traceable. |
| Short screens can clip the composer action row | The message editor, quote and options can scroll inside the composer; the send action stays visible | The keyboard and scrolling history do not hide the next action. |

**Craft report:** verified keyboard and pointer opening/dismissal, actual modal opacity transition events, focus return, field validation and draft persistence, exact stop confirmation, quoted reply navigation, new messages during history reading, and reachable composer controls at 390×675 with reduced motion. The final production asset build and Management browser path passed; the labels fixture passed after the shared interaction layer was introduced. The [render audit](../../.artifacts/management-ui-craft/refinement/accessibility.json) found no WCAG A/AA violations in the five captured states. The detector retained one existing SDK-adapter warning and zero critical findings. The isolated preview serves the reviewed build; existing AgentX deployments and native/API semantics remain unchanged.

[Actual interaction recording](../../.artifacts/management-ui-craft/refinement/interaction.mp4) · [Desktop launch](../../.artifacts/management-ui-craft/refinement/launch-desktop.png) · [Desktop reply](../../.artifacts/management-ui-craft/refinement/reply-desktop.png) · [Dark phone launch](../../.artifacts/management-ui-craft/refinement/launch-mobile-dark.png) · [Dark phone conversation](../../.artifacts/management-ui-craft/refinement/conversation-mobile-dark.png)

## Compact design pass

The user rejected the previous broad controls and heavy surfaces. The interaction work above established behavior; this follow-up establishes compact visual design. It applies UI Craft Distill, Typeset and the selected management brief. The point is not indiscriminate shrinkage: each always-visible field must answer the current question, typography and alignment carry the hierarchy, and boundaries stay for real modal, form, selected and error states.

| Area | Default composition | Disclosure and responsive behavior |
| --- | --- | --- |
| Chrome | Identity, destination, dataset and launch share one low-height desktop line | Phone stacks navigation in a single compact second line; endpoint and generation remain in scope disclosure. |
| Roster | Two aligned text lines for exact name/status and title/membership | Exact working directory moves to the selected subject; mobile index selection stays local. |
| Launch | Compact source choice followed by initial task | Workspace, independent working directory, and title sit behind a summary that surfaces current overrides. An invalid override opens the group and focuses the correct field. |
| Stop | Short consequence plus name, status, and exact execution ID | Cancel retains focus after return; confirm stays the only filled destructive action. |
| Conversation | A measured reading column, small metadata, compact composer | Composer quote/options may scroll internally; intent, options and send remain the action rail. |
| Workspaces | Paths, role declarations, member index and route compose within one readable width | Same component/tokens; discovery coverage remains available and member totals are explicitly loaded-only. |

The [paired capture set and dimensions](../../.artifacts/management-ui-craft/compact/comparison.json) use the same fixture at 1440×900 and 390×675. Desktop chrome changed from 108px to 50px. Launch changed from 620×754px to 460×445px, keeping its required source and task fields directly visible and reducing ordinary height while preserving full scroll on demand. The phone chrome changed from 137px to 89px. The [compact accessibility report](../../.artifacts/management-ui-craft/compact/accessibility.json) reports no axe A/AA findings or page-width overflow in the four captured states. Read its labels against snapshots; accessible labels remain required even when a visible redundant caption is removed.

The compact pass owns its build and checks after existing tasks: public source fixture, semantic labels, validation and keyboard focus, mobile/reduced-motion path, and reviewed desktop/mobile screenshots.

## Compact design pass

The operator rejected the broad controls and heavy surfaces of the previous visual candidate. The interaction tests established behavior; they did not establish visual acceptance. Use UI Craft Distill, Typeset and Remember for this pass. Craft read: calm operator tool, type and alignment first, one muted accent. Judge the result through paired renders and complete actions, not the number of tokens or animations.

| Design decision | Normal state and action | Alternate state and reason |
| --- | --- | --- |
| App chrome | One desktop line: identity, destinations, selected dataset, launch. On phone, scope and launch share the identity line; destination links occupy the second line. | Connection and generation details remain available in disclosure. Save vertical room because this chrome persists while reading and composing. |
| Agent index and selection | Scan two aligned lines: exact name and status, then useful title/membership. Do not print long cwd on every row. Selected row gets an inset tick, not a blue outline. | Cwd and execution facts stay in the Agent's detail. Search covers the same loaded fields. The narrow picker preserves a clear return. |
| Launch | Lead with source choice and the task text. Use a compact text switch and one source field. The position/title disclosure shows any overrides in its collapsed summary. | Reveal and focus an invalid hidden field. Role launches retain their chosen workspace; changing mode retains both draft values. All scope defaults and the optional inputs remain available. |
| Stop | Use one short title, a concise consequence and exact target details as aligned text. Accent is reserved for the confirm action. | Escape and cancel restore focus; native admission still determines the receipt. The dangerous action never gets initial keyboard focus. |
| Conversation | Read on a limited measure with sender, timestamp and reply on one axis. Use a single composer boundary; put recipient and save state on the action rail. | An intentional reply exposes the quote; nested options stay available. Fixed send does not get covered when editing on a short phone. |
| Workspaces and operations | Roles, routing, members and receipts use the same spacing and typography. Reserve borders for actual groups and controls that need a boundary. | Pending and unknown remain labeled. Opening or closing a disclosure never substitutes a visual effect for the underlying result. |

Acceptance: compare the same sample at 1440×900 and 390×675, measuring shell, default launch box, reading area, and reachability. Verify invisible-to-visible launch validation, deliberate stop/cancel, quote and scroll continuity. This compactness contract does not reduce body-text reading contrast or the touch area of high-frequency phone actions.
