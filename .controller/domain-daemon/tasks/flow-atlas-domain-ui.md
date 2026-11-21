---
schema: task
status: completed
description: Build a compact offline domain-model design workbench consuming the shared semantic artifact.
links: [../flow-atlas-workbench-implementation.md, ../../../tools/flow-atlas/design/workbench.md, ./flow-atlas-domain-core.md]
---

# Domain workbench UI

Read root/tool/local UI guidance, Delivery and design. Own `tools/flow-atlas/workbench/**` and this Task only. Shared `spatial/viewport.js/css` and pure `viewer/layout.js` are read-only reusable dependencies. No root manifest/lock, core/tests/generated HTML/server/user/product writes; no execution of Node/Python/build/tests/browser/services/installs. Acceptance is sole executor.

## Structure first and contract consumption

Create a semantic template/shell, named DOM/view modules, tokens and responsive layout FIRST. Core peer owns canonical `domain/contracts.ts` and `domain/index.ts`, with portable `flow-atlas/workbench-1` artifact and precomputed/authoritative scenario checkpoints; consume its actual early release, never guess a second reducer or duplicate business facts. You may send direct contract questions/actionable handoffs to core/acceptance bindings recorded below.

Implement a source-first domain design workspace: compact searchable domain/module/object/rule/process/scenario navigation, large central canvas, contextual dismissible details and bottom player only in scenario mode. Object cards show business identity/kind/responsibility, useful owned state, named public method Port dots and associated rules. Click field/method/rule/parameter line to inspect its Schema, owner, source/metadata/docs and process usage. Cross-navigation retains subject/context. Avoid large standing prose and empty repeated panels.

Use the same semantic IDs and checkpoints for object/relationship, process, dataflow, state and sequence views; a view selector can be compact on narrow screens. Dataflow must actually connect result field/operation input and disclose provenance. State view distinguishes declared legal transitions and visited changes; scenario outcome and caller unknown must not erase remote committed facts. Diagnostics locate actual objects/fields/steps, with honest declared/checkable/tested/unsupported coverage, not fake universal-green proof. Show explicit metadata source/doc text with safe http(s) links.

Player has previous/next/play/pause/replay and adjustable0.25/0.5/1/2 speed (default0.5): visible flow/data marker and local processing phases, prior checkpoint until application, then differences on objects. Pause/view switches retain progress; reset/context cancels. Use one presentation clock. Inputs are inert; browser never invokes native business methods. Source-first editing is through documented TS; do not imply arbitrary graphical semantic writes. Scenario comparison/expected-check navigation should be useful from the actual provided artifact.

Reuse shared container-sized pan/zoom; map fills host with no finite sheet. Relationship/object heading drag, pin, undo/reset and explicit JSON layout exchange should reuse pure existing layout engine where feasible, with world-coordinate correctness and independent presentation state. Preserve actual port endpoints. No general router promise; expose any unresolved obstruction. Keyboard, touch, Escape/focus and390px workspace must remain usable; no body overflow.

`workbench/build.py` embeds the already exported JSON, scripts and shared assets into one offline HTML with exact input/artifact receipt. No user project code executes. Reuse the prior builder's safety patterns/protected sources, avoid dependency install/framework migration. Template slots occur once and inert model/script/style content cannot escape. Do not author an executable server if existing inspector/serve.py can serve the single artifact.

## Handoff

Publish UI module skeleton/build entry and stable browser hooks early. Supply AGENTS/README/source contracts/current limitations and authored UI cases to acceptance; no execution claims. Release all source claims after coherent implementation. Concrete acceptance defects return to this owner.

## Working model and authored structure

The UI is a projection consumer, not a domain interpreter. Navigation and details select canonical IDs; views read the same exported definitions and precomputed scenario checkpoint. A single presentation clock retains the previous checkpoint during request/processing/response and applies the next checkpoint at the end. Scenario/process/reset changes cancel that clock; view/detail changes retain it. Layout and camera are independent presentation state.

| Owner | Responsibility |
| --- | --- |
| `index.template.html`, `workbench.css` | Compact header, searchable resource drawer, central canvas, dismissible detail drawer, scenario-only player; local touch disclosure at390px. |
| `dom.js` | Safe text/SVG primitives, source/doc disclosure and shared ID selection helpers. |
| `views.js` | Object/process/data/state/sequence projections and stable semantic endpoint geometry; no reducer or clock. |
| `details.js` | Schema, ownership, source/docs, parameter provenance, expectations/comparison and diagnostics navigation. |
| `player.js` | One requestAnimationFrame clock with pause/resume and application boundary. |
| `layout.js` | Pointer/keyboard/touch world-coordinate layout editing, pure layout engine exchange, diagnostics. |
| `app.js` | Context, selection and composition; reads actual Core artifact contract before behavior. |
| `build.py` | JSON-only offline embedding, protected inputs, hashes and exact resource receipt. |

Decision: reuse native browser/SVG plus shared viewport and layout engine, without package/framework changes. The existing layout engine accepts relation scope only; workbench stores that scope with process/scenario identity and only allows object relationship movement. Other views retain their own cameras. No general obstacle routing is promised; pure layout diagnostics disclose crossed objects.

Method: layout-first semantic composition from object → method → binding → scenario/checkpoint → expected/recorded state. Core's actual early artifact contract was the skeleton-stage dependency and was received before behavior. Independent acceptance owns executable/rendered evidence; no executable checks have been run by this author.

## Coherent authored UI release

The source-first shell, safe DOM, five same-artifact views, contextual Schema/source/rule/provenance/expectation details, one presentation clock, layout gestures/exchange, offline builder and maintenance/consumer documentation are authored in [workbench](../../../tools/flow-atlas/workbench/README.md). Actual Core `domain/contracts.ts` was received before behavior; local IDs retain parent scopes. The subsequent explicit `completion: reply|local-outcome`, `outcome` checkpoint and compiler-owned `Checkpoint.derived` changes are consumed. Derived availability is read from Core, never recomputed in UI. Initial entry is design mode with no player; scene selection enables default0.5 playback.

The build entry is `python3 tools/flow-atlas/workbench/build.py artifact.json --output output.html`; it imports no project code. Public hooks and authored scenario cases are in the module README. `FlowAtlasWorkbench.state()` returns a detached observation snapshot, not a mutation API. Shared viewport/layout sources remain untouched. No Node/Python/TypeScript/build/test/browser/server/install execution occurred in this source-author Task.

Release to acceptance covers all `tools/flow-atlas/workbench/**` source leaves. Independent execution and rendered evidence remain with [acceptance](flow-atlas-domain-acceptance.md); source authors do not claim those outcomes. Acceptance may return concrete defects directly to this original owner. Limits: source-first semantic edits, sequential exported scenarios, simple authored/default routes with explicit obstruction diagnostics; no arbitrary source inference/semantic writeback/general router. No AgentX or existing demo/runtime was modified.

## Independent readability feedback and bounded repair

Acceptance's [browser-1 result](/tmp/flow-atlas-domain-acceptance-h9uepct1/browser-1/result.json) reports10 behavioral groups passed, including offline,390px, playback, layout/camera, safety, diagnostics and technical reuse. This author inspected its desktop object and dataflow PNGs for information/action density: the resources and local details are useful, but fit at57%/59% reduces field/rule text to roughly6–7CSS pixels. Initial design91% and narrow85% are useful. That concrete evidence justified reopening `app.js` for a readable initial/context camera policy, rather than hiding the second instance or changing domain facts.

Bounded repair: the first visit to a view/process/scenario whose initial shared-camera fit is below85% invokes that adapter's existing100% command, then reveals the current semantic subject. Explicit Fit remains a full overview; a previously entered scope preserves the user's camera. The shared camera source and layout/domain documents remain unchanged. `details.js` also consumes Core's released optional `nativeMethodId`/`nativeOutputSchemaId` fields as inert method/source correspondence and native output Schema. No clock/reducer/checkpoint change occurs. Affected source is released to independent acceptance for focused geometry/visual/continuity and binding-detail checks; source author executed none.

## Accepted UI outcome and closure

The original independent [acceptance Task](flow-atlas-domain-acceptance.md#accepted-final-derivatives-and-publication) is complete for the documented Linux local/static-browser scope. Its [final handoff](/tmp/flow-atlas-domain-acceptance-h9uepct1/final-handoff.json) and [exact source snapshot](/tmp/flow-atlas-domain-acceptance-h9uepct1/source-final.json) own the accepted facts. Initial10 browser groups passed; the final7 affected groups additionally establish native method details, readable scenario/dataflow entry at100%, explicit Fit/revisit behavior, pause/view/processing continuity, committed local unknown,390px keyboard/touch and technical reuse. Prior unaffected layout/safety/diagnostic/outcome evidence remains accepted by reference. The source author did not execute those checks.

Acceptance actually inspected final desktop/narrow images: primary object state/methods and parameter sources remain readable; secondary content stays reachable through camera/resource navigation. The original57%/59% readability defect is resolved. Native input/output/method correspondence, zero static invocation, semantic/native boundaries and exact publication checks remain owned by the acceptance Task; no pending UI defect remains.

The accepted [offline domain example](../../../tools/flow-atlas/examples/domain-workbench.html), its [model](../../../tools/flow-atlas/examples/domain-workbench.json)/[receipt](../../../tools/flow-atlas/examples/domain-workbench.receipt.json), and the [technical example](../../../tools/flow-atlas/examples/domain-workbench-technical.html) are published. The intentional demo is [http://192.168.2.178:18959/](http://192.168.2.178:18959/); exact process/artifact ownership stays in the [server record](../flow-atlas-workbench-server.json). The tested and published artifact bytes match offline/loopback/LAN. Preserve this demo and earlier demos/user runtimes.

This Task is completed and all UI source/documentation claims are released. Controller owns broader routing and formal Delivery closure. Only receiving documentation changed after final acceptance; executable/template/builder/style/script and generated inputs remain frozen. The authored case table in the workbench README remains distinct from actual independently executed evidence linked above. Limits remain finite sequential exported design, source-first semantic changes, presentation-only layout and simple routes; no arbitrary inference/writeback/general router/live runtime support is implied.
