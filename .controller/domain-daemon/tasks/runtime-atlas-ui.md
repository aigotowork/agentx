---
schema: task
status: completed
description: Implement compact process map, local inspector and scenario player against the canonical atlas model.
links: [../runtime-atlas.md, ../../../.docs/architecture/runtime-atlas/README.md]
---

# Runtime atlas UI

Read root/shared/local guidance and local interface standard, then Delivery and atlas README. The model contract below is released for consumption; request concrete additive changes directly rather than guessing product facts.

## Ownership

Own only `.docs/architecture/runtime-atlas/{index.template.html,atlas.js,atlas.css,layouts/workspace.css}` and this Task. Controller owns model, builder/generated output, tests/browser/server, guides and all executable checks. Do not run Node, Python rendering/build, browser, formatter, Cargo, dependency installs or product commands. Do not delegate. Publish a usable composition skeleton early and release coherent source directly via AgentX. No SVG approval stage: this is the user's requested operable explanatory artifact.

## Frozen consumer contract

Template contains `__ATLAS_STYLES__`, `__ATLAS_MODEL__`, `__ATLAS_SCRIPT__` exactly once. Put model inside `<script id="atlas-model" type="application/json">` and JS in a plain classic script. Builder embeds both CSS files and model/script. JS reads JSON.parse from that element; no imports/network/external assets/framework/API. Prefer inline SVG with selectable processes/modules and labeled transport edges. Use safe text rendering.

Model: `{schemaVersion,title,scope,sources,nodes,edges,modes,scenarios}`; collections are arrays. Node: `{id,label,kind,owner,executable,lifetime,capabilities,modules:[{id,label,path,description}],sourceIds}`. Edge: `{id,from,to,label,transport,purpose,sourceIds}`. Mode: `{id,label,command,summary,nodeIds,edgeIds,initial:{nodes:{[id]:state},facts:{[key]:text}}}`. Scenario: `{id,modeId,label,question,initial:{nodes,facts},steps:[{id,title,actor,handler,edgeIds,actions,event,patch:{nodes,facts},sourceIds,certainty,note}]}`. State is `{label,tone}`; tones `neutral|running|busy|waiting|stopped|unknown|stored`. Source: `{id,label,path,basis}`. Facts keys `connection,host,provider,turn,inbox`; use stable Chinese UI labels. Missing patches preserve values. Reconstruct steps by folding initial plus patches; index=-1 is before action. Node/edge references belong to that mode. Modes `foreground|headless|tmux|appserver|workbench`. Semantic model is controller-owned; private view coordinates belong in UI.

## Interaction

Main screen prioritizes resource relationships; local inspector exposes process ownership/lifetime/capabilities/modules/source paths or edge transport/handler. Highlight current step actor, handler and edges. Show its actions plus five distinct state facts. Use compact controls, no giant explanation cards. Modes/scenarios use native selects if helpful. Support previous/next/reset and optional autoplay, reduced motion, focus/click/tap selection for nodes/modules/edges. Default appserver/disconnect. Hash keys `mode,scenario,step,node`; invalid refs fall back coherently. Mode changes reset incompatible playback/selection. Narrow390px fits controls/inspector without document-wide horizontal overflow; map may scroll inside its viewport. Visible `行为演示` or `架构演示` label; no live-status claims. Copy source paths with an insecure-LAN fallback.

Stable `data-testid`: `mode-select`, `scenario-select`, `step-prev`, `step-next`, `step-reset`, `step-play` if present, `map-node-{id}`, `map-edge-{id}`, `module-{id}`, `inspector`, `step-detail`, `facts`, `step-counter`, `atlas-map`. Icons have accessible names and keyboard/touch disclosure. Author meaningful proposed browser cases in this Task; execution belongs to controller. Release all writes/resources on handoff.

## Source implementation handoff

The bounded UI source is released in the four owned presentation leaves: [index.template.html](../../../.docs/architecture/runtime-atlas/index.template.html) owns the portable semantic shell and the three builder slots; [atlas.js](../../../.docs/architecture/runtime-atlas/atlas.js) owns model projection, folded playback state, hash restoration, selection, inspector disclosure, source copying and autoplay; [atlas.css](../../../.docs/architecture/runtime-atlas/atlas.css) owns tokens, component states and inline SVG appearance; [workspace.css](../../../.docs/architecture/runtime-atlas/layouts/workspace.css) owns the desktop/narrow workspace geometry. The source consumes the released `model.json` contract without adding semantic facts or network/runtime dependencies.

The main screen keeps the relationship map primary: each mode renders its declared nodes and edges, each node contains its declared modules, active scenario edges and actor receive a local visual emphasis, and the inspector exposes the selected node, module or edge with ownership, lifetime, capabilities, transport, handler context and source paths. Scenario playback folds `scenario.initial` and prior patches; `mode.initial` is used only as the declared fallback when a scenario has no initial state. The step detail keeps `note` contextual to the current event, while the facts region always renders the five independent contract keys: connection, host, provider, turn and inbox.

The browser-facing state contract is implemented without external assets or APIs. Invalid hash references fall back to the appserver/disconnect starting view or the first compatible mode/scenario; mode changes reset incompatible playback and selection. Nodes, modules and edges are selectable by click/tap and Enter/Space, controls have stable test IDs and accessible names, source paths use the Clipboard API when available and a textarea `execCommand` fallback on insecure LAN pages, and the map owns its narrow-screen horizontal scroll so the document remains within the viewport. The page labels itself as a behavior demonstration and keeps unknown facts from becoming live-status claims.

## Proposed browser cases for controller execution

These are authored cases for the controller's independent browser acceptance; no execution is claimed by this Task.

- Open the generated page without a hash and confirm `mode-select` is `appserver`, `scenario-select` is `appserver-disconnect`, the map shows the declared appserver nodes/edges, and the initial facts come from the scenario's pre-action state.
- Select the first scenario step with `step-next`; confirm `step-counter`, `step-detail`, the actor/handler chips, the current event edges, the folded node labels and all five fact cards update together. Use `step-prev` and `step-reset` to return to the scenario initial state.
- Select a process, one contained `module-{id}`, and one `map-edge-{id}` by pointer and keyboard; confirm `inspector` changes between process ownership/lifetime/capabilities/modules/sources, module path/description, and edge transport/purpose/source details without losing the mode or step.
- Choose each mode and a compatible scenario; confirm switching mode resets step and selection, updates the declared node/edge subset, and never presents a scenario from another mode. Load invalid `mode`, `scenario`, `step` and `node` hash values and confirm coherent fallback.
- Advance to the appserver disconnect step whose note describes actual stdio closure; confirm that note appears inside `step-detail` only, while the model's unknown or scoped facts remain visible in `facts` and no live-status wording appears.
- Copy a source path in a secure context and in an insecure-LAN context; confirm the control reports the copy result and no source value is inserted into markup unsafely.
- At a 390px viewport, confirm mode/scenario/step controls remain reachable, the inspector and step detail are readable, the map scrolls inside `atlas-map`, and the document has no horizontal overflow. With reduced motion enabled, confirm playback remains operable without required animation.

Controller-owned integration remains: `model.json`, builder/generated output, browser/server operation, executable checks and final acceptance. This Task performed only static source inspection and releases its presentation writes/resources here.

## Rendered desktop correction

The controller's [overview screenshot](/tmp/agentx-runtime-atlas-9t51ayf1/browser-r1/01-overview.png) and [disconnect screenshot](/tmp/agentx-runtime-atlas-9t51ayf1/browser-r1/02-disconnect.png) at 1440×900 establish a substantive workspace failure: the oversized map pushes event actions and some state facts below the viewport; the 1240-unit diagram scales module/path text down; long node states share the title line and overlap adjacent content. These images answer the user-intent question negatively for the first source handoff. Existing controller functional acceptance remains separate and does not establish visual readability.

The bounded correction keeps existing playback, selection, model and test IDs: use a viewport-budgeted desktop workspace with independently scrollable detail panes; compact diagram coordinates and readable module-only rows; separate title and state rows with conservative visual truncation and full source text in accessible labels/tooltips/inspector. The current model also distinguishes `process-group` explicitly and relocates executing modules to their actual owners; UI reads that data without adding ownership facts. Only the affected presentation source and this Task are authorized for this correction; the controller retains browser/build/model/test execution and the affected rendered acceptance.

### Scoped source release

The correction is released in `atlas.js`, `atlas.css` and `layouts/workspace.css`; the HTML template and all model/build/generated/test/server files are unchanged by this worker. Desktop layout now reserves a 264px event/fact band before assigning remaining viewport height to the map and inspector, with local overflow for map and detail panes. Horizontal page padding is 24px, the inspector is 320px, and all five fact keys occupy compact labeled rows. The 1440×900 source budget leaves approximately 400px for diagram content after headers, without displacing the event/fact band. This is a structural budget, not a new rendered-pass claim.

The large-mode diagram is 1044×404 at its natural pixel width; workbench uses 1044×246. Private view coordinates and transport lanes replace the previous 1240×650 view and its forced 520px minimum height. Nodes size from their actual module count; title, kind/state and module rows are separate. Map typography is 13px for titles, 12px for module names, 11px for states/transports and 10px for kinds. Source paths remain available through full accessible module labels, SVG titles and the local inspector, while the map paints module names only. Visual truncation preserves the original model strings in the accessible/inspected surface and does not substitute a new status meaning. Transport labels use the model's actual `transport`, with the operation label retained in accessible titles and the inspector. `process-group` maps to “进程组” with an explicit kind label and dashed outline.

The fold, hash, keyboard/touch handlers, actor-selection-on-step behavior and stable test IDs were left intact. Narrow layout retains a map-local horizontal viewport and stacks bounded inspector/event panes; facts use the same five labeled rows. No browser, build, formatter, model validation or executable check was run by this worker. The only visual inspection was the two controller-supplied desktop screenshots, followed by source reading. All source writes/resources are released to the controller for the affected desktop/narrow render and behavior checks. The original first-source release above is historical; this release supersedes its geometry/readability assumptions without converting earlier functional results into validation of these new bytes.
