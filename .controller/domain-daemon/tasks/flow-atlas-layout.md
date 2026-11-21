---
schema: task
status: completed
description: Implement independent draggable layout editing in the existing offline Flow Atlas viewer.
links: [../flow-atlas-implementation.md, ../../../tools/flow-atlas/design/README.md]
---

# Usable canvas layout editing

Read root/tool AGENTS, shared/local interface guidance, parent Delivery and target design's spatial contracts. Implement the accepted layout slice inline in existing SVG viewer; no React rewrite/dependency install. You own viewer/** (including optional new layout.js), tool build.py source registration only, and this Task. Controller owns runtime consumer build wrapper/HTML, shared docs/model/tests and all execution. Other specialist owns TS authoring and design files. No overlap without explicit transfer.

## Required outcome

Separate pure layout/session geometry from DOM interaction. Drag only object heading/handle, pointer threshold/capture, Escape/pointercancel restores original point, one undo entry per committed gesture. Pin/unpin, undo/reset, keyboard movement with clear accessible action, layout JSON export and preferably strict import. Pause playback on actual drag, retain elapsed progress and remain paused. Switching scenario/view/reset cancels an active drag. Keep Port/state clicks distinct and usable by touch. Preserve finite snapshot/model semantics. State layout by content identity/view/scope; no assumed localStorage or product persistence.

Routes must follow moved actual Port coordinates including reverse replies. Do not introduce or claim a generic obstacle router without established mechanism. Detect obstructed/unresolved routes and render explicit contextual diagnostics; do not silently move pinned nodes or fake routing success. Automatic arrangement and routing are distinct. Existing accepted geometry should remain default when no edits exist. Keep UI compact: canvas primary, small layout actions/local disclosure,390px internal scroll/no page overflow. Metadata and authored traces unchanged.

Provide stable testids and callable pure layout API early for controller's independent tests. Author important pointer/touch/keyboard/playback/route/export case notes, but do not execute Node/Python/build/browser/tests/formatters/product commands. Controller handles generation and verification. Do not edit live HTML/server record or operate listeners/.agentx/.bin/Provider/installed resources. Do not spawn. Final handoff lists exact leaves, contracts, limits and releases all writes.

## Working model

The assigned layout specialist owns this bounded spatial slice; no nested delegation or execution. Source inspection confirms an existing pure canvas geometry layer plus a DOM/clock owner in `atlas.js`. Preserve the default authored geometry and add three explicit responsibilities: `viewer/layout.js` owns immutable sessions, content-bound layout exchange and geometry diagnostics; `viewer/layout-ui.js` owns pointer/keyboard/file interaction and local disclosure; `atlas.js` composes these with existing projection/playback APIs. `build.py` only registers the new scripts. The existing [spatial design](../../../tools/flow-atlas/design/README.md#线port-和标签) is a target; this slice does not add automatic arrangement, arbitrary obstacle routing, durable storage, manual route editing, pan/zoom or another view's object editing.

The session transition is `idle → pressed (DOM only) → dragging (pure gesture) → commit/cancel → idle`. Only crossing a six CSS pixel threshold pauses playback. A pure gesture retains the original document and commits at most one history entry. Pinning blocks movement until explicitly unpinned. Mode/scenario/view/checkpoint/reset, pointer cancellation, lost capture, window blur and Escape cancel the gesture. Layout sessions are keyed by SHA-256 semantic content identity and exact relation-view/mode/scenario scope; imported files must match that identity, scope and complete node set. The viewer v1 layout document has its own explicit format, rather than pretending to implement the broader target TS `LayoutDocument`.

Routing and placement remain separate. Call geometry uses measured-in-model card boxes and actual inbound pin positions; both forward requests and reverse replies share that same recomputed polyline. Context transport paths retain authored hints until their endpoint moves, then use the existing fallback. Segment/card intersections and overlapping boxes produce local diagnostics. Detection does not claim obstacle avoidance and never repositions pinned nodes.

## Independent acceptance cases (controller-owned execution)

| Case | Expected result |
| --- | --- |
| Pure begin/update/commit and begin/update/cancel | Immutable caller input; many updates add one undo item, cancel restores exact original, zero displacement adds none. |
| Pointer heading versus Port/state | A heading press below threshold remains a selection; Port and state clicks never begin dragging; touch heading has the same behavior. |
| Actual drag during transfer | Pause at retained phase/elapsed progress; card and inbound pin move together; request path ends at the moved pin and reverse response follows the same path; release remains paused. |
| Escape, pointercancel, lost capture, blur and view/scenario/reset | Original point restored, no history added, no stale pointer later commits. |
| Keyboard and touch alternatives | Heading arrows move ten model units, Shift+arrow one; P pins/unpins; local directional buttons move the selected object; pin prevents movement. |
| Undo/reset/scope | One undo restores one drag, reset is undoable, scenario/view return restores its in-memory scope without changing model/checkpoints. |
| Export/import | JSON carries format/content digest/view/mode/scenario; unchanged canonical model identity survives key ordering; stale content, other scopes, unknown fields/nodes, duplicate pins and nonfinite coordinates fail without losing current layout. |
| Obstructed route/overlap | Diagnostic identifies route/node and remains visible on the path and in local disclosure; pinned positions stay fixed. |
| 390px viewport | Main document stays within viewport; canvas scrolls internally; local layout actions and diagnostics remain reachable. |

## Source handoff and write release

Implementation is complete for independent controller acceptance; no execution evidence is claimed by this specialist. All source writes are released. Exact changed leaves: `tools/flow-atlas/viewer/layout.js`, `tools/flow-atlas/viewer/layout-ui.js`, `tools/flow-atlas/viewer/atlas.js`, `tools/flow-atlas/viewer/canvas.js`, `tools/flow-atlas/viewer/atlas.css`, `tools/flow-atlas/viewer/layouts/workspace.css`, `tools/flow-atlas/build.py` (script registration/order and its matching docstring only), and this Task. No other source, authored model, generated/live HTML, receipt, listener, product runtime or installed resource was changed. No Node, Python, builder, formatter, test, browser or product command was executed.

The canonical callable contract is [FlowAtlasLayout](../../../tools/flow-atlas/viewer/layout.js), a classic-script global with `identity`, `create`, `begin`, `update`, `commit`, `cancel`, `move`, `togglePin`, `pinned`, `undo`, `reset`, `exportDocument`, `importDocument`, `project`, `overlaps`, and `routeDiagnostics`. `create({contentDigest,scope:{view:'relations',modeId,scenarioId},positions})` returns `{document,defaults,history,gesture}`. Update/move coordinates are absolute model coordinates. Functions do not mutate inputs. Exchange uses `flow-atlas/viewer-layout-1`, exact canonical SHA-256 content, exact view/mode/scenario and the complete node map. Coordinates must be finite within ±1000000; duplicate/unknown pins, unknown fields, foreign scopes and stale content are rejected atomically. Import is undoable and export excludes an uncommitted gesture. This is the existing finite viewer's exchange format, not implementation of the target TS platform's richer `LayoutDocument`.

[FlowAtlasLayoutUI](../../../tools/flow-atlas/viewer/layout-ui.js) composes the pure API through injected model/context, pause, selection and repaint hooks. Stable test IDs: `layout-handle-{nodeId}`, `layout-controls`, `layout-toggle`, `layout-menu`, `layout-pin`, `layout-undo`, `layout-reset`, `layout-export`, `layout-import`, `layout-import-input`, `layout-move-{left|up|down|right}`, `layout-feedback`, `layout-diagnostics`, and repeated `layout-diagnostic` rows with `data-diagnostic-code` and optional `data-route-id`. `atlas-map` exposes `data-layout-dragging` and `data-layout-undo-count`; `map-node-{id}` exposes `data-layout-pinned`; existing edge/call groups expose `data-route-status`. A six CSS pixel threshold starts a drag; capture belongs to the stable map region. The viewBox stays fixed during preview, then expands after commit. Arrow keys move 10 units, Shift+arrow 1; P toggles pin; Enter/Space retain object inspection. The local directional controls provide a touch alternative. Import has a 1 MiB file limit and rejects an asynchronous result if the scope or draft changed while reading.

Important limits for the controller's shared documentation update: layout edits apply only to the relation canvas; flow and sequence remain explanatory projections. Layouts live only in page memory and require explicit JSON export for saving. No automatic arrangement, generic obstacle routing, manual bend editing, browser storage, pan/zoom or target-platform SDK support is added. Simple polylines follow current actual inbound pins, including reverse call replies. Obstruction/overlap detection adds contextual path/node styling and a local diagnostic list; it does not reroute successfully by assertion. Authored context route hints expire only when their endpoint moves; untouched default geometry stays unchanged. The controller owns updates to tool AGENTS/README/contracts/current-support records and actual generation/pure/browser acceptance using the cases above.

## Independent acceptance received

The controller completed affected executable/browser acceptance and integrated this source; see the singular [accepted result](../flow-atlas-implementation.md#accepted-local-slice). Original author-only evidence above remains historical, and no source/resource claim is active.
