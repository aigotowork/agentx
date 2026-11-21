---
schema: task
status: completed
description: Build the polished interactive SVG/HTML contract canvas over the canonical typed model.
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"nuvi","incarnation_bits":4745315424684224087}}]
---

Read the Delivery, Proposal, model Task output, completed Atlas evidence, interactive model communication and interface requirements. Own the portable artifact under .docs/object-atlas-v2/ and evidence under .controller/object-atlas-v2/evidence/; do not edit the old Atlas, SDK, native source or live services.

Build a real object-oriented workbench, not a static graph. The overview shows domains and major types. Selecting a type opens a class/interface panel with fields and method ports. Selecting a port reveals typed input/output/error edges. Selecting a type reference navigates to that type while preserving origin context. Provide focused mode, upstream/downstream tracing, search, breadcrumbs, reset/overview and a compact scenario lens.

Include a dedicated MemberPage type-detail scene and a Workspace.members method-trace scene. Include the send and Operation recovery sequence. A method port must be visually distinct from a type node; relation direction must be obvious without prose. Keep SVG geometry, orthogonal routing, stable selection, semantic focus, keyboard activation, narrow layout and no overlap or overflow. Model data and renderer must be separate files. Test desktop, narrow, direct offline opening and keyboard. Capture entering MemberPage from Workspace.members, then MemberCursor, AgentSnapshot, Agent and returning by breadcrumbs.

## Canvas design and working state

The reader is an SDK consumer tracing a returned contract and recovering an ambiguous send. The decisive question is whether `Workspace.members → MemberPage → MemberCursor / AgentSnapshot / Agent` is navigable as first-class objects with clear typed ports and a continuous return path. The complementary path is prepared send → response loss → retained operation → observe/wait → receipt branches.

Selected composition: light technical drafting surface with domain bands, structural class cards, dark method-port bars, directional orthogonal edges and a persistent compact inspector. A free node cloud obscures ownership; a table-only reference obscures typed flow. Four coordinated scenes share stable identity: domain overview, type detail, method trace, recovery sequence. Desktop uses navigator / Canvas / inspector; narrow uses the same Canvas with local navigator and inspector drawers.

Owned files: `.docs/object-atlas-v2/index.html`, `canvas.css`, `canvas.js`, `layout.js`, `README.md`; canonical facts remain in the model owner's `model/` files. No runtime calls, effect simulation, SDK changes or old Atlas reuse. Keyboard activation, search, breadcrumbs, tracing, focus, fit, zoom and source disclosure are part of the batch. Model API handoff requested from @labu.

Initial uncertainty: model API is being established concurrently; renderer shell and layout can proceed independently, final binding waits for the owner. Browser host is local Chromium through existing Playwright package if available.

## Completed Canvas outcome

The [portable Contract Canvas](../../../.docs/object-atlas-v2/index.html) is complete and remains available at http://192.168.2.178:18988/. The [renderer handoff](../../../.docs/object-atlas-v2/README.md) owns exact file responsibilities, page/Layout/Block/Component inventory, navigation/trigger map, original vector assets, fixture/reset behavior, motion and run instructions. The [local entry](../../../.docs/object-atlas-v2/AGENTS.md) routes future changes to renderer, layout and model owners.

Final owned renderer files are `index.html`, `canvas.css`, `layout.js`, `canvas.js`, `README.md` and `AGENTS.md` under `.docs/object-atlas-v2/`. The model owner's separate [canonical model](../../../.docs/object-atlas-v2/model/README.md) supplies all facts and source status through the generated offline wrapper; no canonical model file was edited by the Canvas owner. No old Atlas, SDK, native source or existing live service was changed. The dedicated static preview uses its own port and remains running.

The initial complete composition exposed two material issues: scaling the complete native dependency closure made overview labels unreadable, and a fixed reference-row height omitted the third `MemberPage.members` reference. The corrected overview gives 16 major types and distinct dark method ports four coherent domain regions, with all 119 non-method types and all 20 ports reachable through directory/type ownership/search. Field rows expand for all typed references. Error/outcome groups preserve navigation without stretching the method scene into unreadability. The valid `MemberPage.coverage → MemberCoverage.next → MemberCursor` route is preserved; no direct page-to-cursor relation is invented.

The final geometry correction routes desktop edges through rectangle-clear orthogonal channels; narrow type edges use the central gutter and method edges use side channels. The routing follow-up was limited to affected scenes and did not repeat the previously passed full walkthrough. Keyboard navigation now moves focus into the selected class panel so its field references follow directly in tab order.

## Evidence

| Evidence | Observed result |
| --- | --- |
| [Browser report](../evidence/browser-report.json), [reproduction](../evidence/browser-check.cjs) | Passed 1440×900 desktop, 390×844 narrow, typed input/result/cursor/snapshot/Agent investigation, breadcrumbs, focused/trace/camera controls, sources, seven recovery steps, five native receipt statuses, local observation stop, deferred native paging, keyboard, drawers and direct `file://` opening while browser network was offline; no page errors or business requests |
| [Geometry report](../evidence/geometry-report.json), [reproduction](../evidence/geometry-check.cjs) | Passed all four desktop scenes and narrow method/type/recovery scenes after routing correction; no sampled edge intersection with unrelated card interiors, no document horizontal overflow, no page errors |
| [Focus report](../evidence/focus-report.json), [reproduction](../evidence/focus-check.cjs) | Search submit → method port; Enter → MemberPage transfers focus into its class panel; Tab → first field; Space → MemberEntry |
| [Final artifact manifest](../evidence/artifact-manifest.json) | SHA-256 identity of final renderer and consumed model bytes |

The required captured path is [MemberPage entered from Workspace.members](../evidence/02-memberpage-from-members.png), then [MemberCursor through coverage.next](../evidence/03-membercursor.png), [AgentSnapshot](../evidence/04-agentsnapshot.png), [Agent](../evidence/05-agent.png), and [breadcrumb return](../evidence/06-breadcrumb-return.png). Final composition captures include [overview](../evidence/01-overview.png), [method port/capabilities](../evidence/07-method-port-and-capability.png), [send/Operation recovery](../evidence/08-send-operation-recovery.png), [native Unknown branch](../evidence/09-native-unknown-branch.png), [narrow MemberPage](../evidence/10-narrow-memberpage.png), [narrow recovery](../evidence/11-narrow-recovery.png) and [narrow method](../evidence/12-narrow-method.png).

Visual observation: the four-region overview remains readable at natural desktop fit, method terminals differ by shape/fill from type cards, all three member-row references are visible and keyboard reachable, and narrow screens keep subject/context/controls while the diagram scrolls locally. Geometry sampling supports the captured routes, not arbitrary future layouts. Initial `first-*` captures are intermediate evidence and do not own the final outcome.

## Limits and resolved friction

This is finite author-operated browser and source-linked presentation evidence. It does not establish independent user research, exhaustive source equivalence, runtime availability, registry publication, native message effects, delivery/read/model execution, or production adoption. Native Workspace-filtered paging remains first-class and explicitly deferred. Receipt `Unknown`, local `OutcomeUnknown`, and local `ObservationStopped` remain distinct; the artifact executes none of them. Controller owns Delivery acceptance.

Model handoff initially preceded the actual JSON write; that uncertainty was reported and resolved with the model owner's validated JSON plus generated classic-script wrapper. The initial browser script selected the narrow navigator's hidden close control rather than its visible opener; the selector was corrected and the complete requested path then passed. One geometry capture launch lost its Chromium context; a fresh bounded geometry run passed without altering other browser processes. No unresolved blocker remains for this scoped Canvas assignment.
