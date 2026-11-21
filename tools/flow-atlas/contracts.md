---
schema: design
status: active
owner: flow-atlas
description: Object-owned ports and explicit call/outcome/effect projections for authored traces.
links: [./README.md, ./model.py, ./core.js, ../../.controller/domain-daemon/tasks/runtime-atlas-ports.md]
---

# Objects, ports and interactions

This document remains the implemented v1 contract. The broader user-requested tool is defined separately by [the target product model and TypeScript declarations](design/README.md), which distinguish type definitions, process graphs, concrete scenario traces and presentation layout. Current implemented authoring, Inspector, node editing and shared viewport support are tracked by the [implementation Delivery](../../.controller/domain-daemon/flow-atlas-implementation.md); broader SDK and routing targets remain separate.

## Scope and structure

The user selected a behavioral boundary model: objects expose ports, other objects interact through them, and observable outcomes may change state. Internal computation stays opaque. Preserve the existing offline authored-trace tool; do not add business execution, inferred transitions, a schema migration service, a plugin system or a workflow engine. Object kinds and process/module containment remain independent from capability contracts.

Dependency: consumer objects/ports/trace → structural validation → pure fold/diff/calls projections → relationship/flow/sequence viewers sharing one presentation clock. Existing `initial` and `patch` remain the single authority for snapshot values. Interactions reference those patches through effects; never duplicate next-state values in a second reducer. Original traces without ports remain valid and explicitly show their existing channel/actions.

## Selected additive schema (version 1)

`ports[]` is optional (default `[]`). Each port is an inbound capability owned by a node, not a transport socket or a claim that an external process exposes the exact symbolic name as a native API. The model supplies the label and source basis. One operation per port is sufficient for this release.

| Port field | Contract |
| --- | --- |
| `id`, `nodeId`, `label`, `kind` | Globally unique safe ID, existing owner node, display operation name, `command|query|event`. Event means an input handler for a fact emitted by another object. |
| `input` | List of `{name,type,required}`; descriptive parameter contracts. Safe unique parameter names, nonempty type text, boolean required. No executable expression or type DSL. |
| `outcomes` | Nonempty list of `{id,label,kind}`; safe IDs unique within the port; kind `accepted|returned|observed|rejected|unknown`. These are allowed observable outcomes, not promises that every invocation succeeds. |
| `description`, `sourceIds` | Optional string/list; normalized to empty. Generic tool never opens source paths. |

`step.interactions[]` is optional (default `[]`). Each entry is `{id,from,portId,requestBeat,responseBeat?,input,result,effects}`:

- `id` is unique within its step. `from` and the port's owner must be visible in the current mode. `portId` resolves the target node; do not duplicate a competing `to` field.
- `requestBeat` indexes an authored transfer beat; its actual source/target, including reverse direction, must match `from` and `port.nodeId`. A local beat is allowed only for a self call at that same node. A disconnect/retain is not an invocation.
- Optional `responseBeat` is a later transfer on the same channel in the opposite direction, returning to the original caller. No response is inferred when this annotation is absent. Unknown outcomes cannot claim a response. Successful calls may have no separately drawn reply; outcome knowledge is still disclosed only at the applied step checkpoint.
- `input` is a JSON object with exactly the declared parameter names (optional parameters may be absent); required keys must be present. Values stay inert JSON; the descriptive `type` does not run a validator or coerce data.
- `result` is `{outcomeId,summary?}` referencing one of the port's declared outcomes. Optional summary defaults to the outcome label. The result is the authored outcome at the applied checkpoint, not evidence from a live invocation.
- `effects` defaults to `[]`. Entries `{target:'node'|'fact',id,kind:'change'|'observation'}` reference that step's patch and must correspond to an actual changed value. Resolve before/after through the existing pure core. Rejected/unknown calls may have observational effects but must not claim successful `change` effects. They do not prove that no remote effect happened. Observation means an update to what the model knows; global facts remain projections, not invented native ownership.
- A query may return an observation without mutating the target domain; temporary processing state can still be drawn by the authored step. The tool validates references and explicit result/effect meaning, not hidden business logic or native transaction atomicity.

## Public projections

Existing `FlowAtlas.fold`, `changes`, `defaultSelection` stay unchanged. Add `FlowAtlas.calls(model,scenarioId,index)` returning fresh resolved entries:

```text
{id,from,target,port,requestBeat,responseBeat?,input,
 result:{id,label,kind,summary},
 effects:[{target,id,kind,label,before,after,beforeTone?,afterTone?}]}
```

`from` and `target` are node objects; `port` is the port object. Input/model/projection values remain independent; `-1` returns `[]`, invalid scenario/index follows existing RangeError semantics. The core owns no clock, DOM or I/O. Sources and effects are inspectable without evaluating user code.

## Experience and observable stages

### Canvas projection

Nodes may optionally declare `stateFacts: [factId, ...]` (default `[]`), unique references to declared facts. These identify useful object-associated state projections; they do not prove native storage ownership. The viewer reads values directly from the canonical folded snapshot and computes displayed differences from existing changes. No value or second transition definition is stored in the node. Ordinary lifecycle label/tone remains visible independently.

The relationship canvas renders named inbound Port rows, each with an edge dot, and connects scenario calls to those actual Port anchors. The caller endpoint is a hollow call-origin marker unless a source capability is explicitly modeled; it does not invent an outbound callable Port. The background channel remains transport context and remains authoritative for unannotated or disconnect beats. A declared reverse response follows the same call path back to its caller. Moving data labels consume inert input, and show a pending response until the checkpoint is applied. State fields stay prior during flow/handling and highlight actual before→after differences only at application. Full input/outcome/source details remain available locally; map objects remain the primary source of structure.

Relations show an object-owned compact port affordance and active capability on the relevant connection. Selecting a port opens its contract locally. Flow cards show their primary operation and explicit authored outcome at completed checkpoints; future outcomes are not marked as already known. Sequence rows show the exact request port and, when authored, a distinct response arrow/result. Existing disconnect symbols remain disconnections and do not become invented backend stop methods.

The event dock shows the current caller → target.port, command/query/event kind and phase (`未调用 → 调用中/等待结果 → 返回中 → outcome`). Terminal result and referenced effects are only presented as known when `phase=applied`; planning details before that point must be labeled pending. View/rate switching, pause/replay, keyboard/touch, compact map-first layout and offline behavior remain.

## Concrete outcome slice and limits

AgentX runtime source grounding covers message submission/persistence, claim, native input issuance, native matching-record event and delivery confirmation, plus front-end disconnect. Accepted submission is separate from Provider delivery and Agent business completion. Preserve current native state semantics; a declared port label is an explanatory grouping of source-owned behavior, never an invented wire method.

The unrelated catalog example covers command/query/event input ports and explicit success, rejection and unknown branches as separate authored scenarios. No branch solver or live backend is added. Acceptance must reject mismatched owner/direction, missing parameters, illegal response beats, unknown outcome references, effect references outside patches and rejected/unknown successful effects; actual browser evidence must establish delayed outcome visibility and synchronized views.

## Independent viewer layout

### Shared spatial viewport

`spatial/viewport.js::FlowAtlasViewport` is shared by the trace viewer and design inspector. It owns screen navigation only; domain snapshots, object position edits, illustrative time and compiler diagnostics have separate owners. The host DIV determines screen width/height, and its SVG fills those exact dimensions. Content `getBBox()` supplies initial/explicit fit only; it never defines a finite sheet or panning boundary.

Public surface: `zoomAt(camera,zoom,screenPoint)` and `fit(bounds,size)` return new `{x,y,zoom}` cameras; `create({host,controls,beforeNavigate?})` returns `{mount(svg,scope),fit,reveal,state,dispose}`. Camera x/y are world-space top-left; zoom is CSS pixels per world unit, bounded0.1–4. `mount` borrows the inserted SVG and retains an in-memory camera per consumer scope. `state` returns a copy. Resizing retains center and scale. `dispose` removes owned listeners, controls and ResizeObserver. No network, domain mutation or persistence is performed.

Wide hosts initially fit content without exceeding100%; below600CSS pixels initial zoom is at least85%, anchored to content top-left to preserve readable objects. Explicit fit always requests a full overview. Blank primary drag, middle/Space drag, pointer-anchored wheel zoom, touch blank pan/pinch, keyboard arrows/zoom/100%/fit and the overlay controls share this camera. Escape or pointer cancellation restores a gesture's initial camera. Object click and heading-edit ownership remain separate; `beforeNavigate` cancels an active node gesture before its world coordinate frame changes. Camera values are not exported in the existing node-layout JSON.

The common CSS keeps the world-coordinate grid continuous across the viewport and overrides content-sized SVG/margin rules. Each builder embeds these shared sources; consumers must regenerate HTML after source changes. Browser outcome cases live at `tests/flow_atlas_viewport_browser.mjs`; [the viewport Task](../../.controller/domain-daemon/tasks/flow-atlas-viewport.md) owns exact checked artifacts, desktop/touch geometry and navigation evidence.

### Node position document

The implemented v1 spatial contract is `viewer/layout.js::FlowAtlasLayout`, consumed by `viewer/layout-ui.js` and the existing playback host. Its `flow-atlas/viewer-layout-1` document is independent of the broader target TS LayoutDocument: `{format,contentDigest,scope:{view:'relations',modeId,scenarioId},positions,pinned}`. Identity is canonical-content SHA-256, and imports require the exact scope, node set, strict fields and finite ±1000000 coordinates. Unknown/stale layouts fail atomically. Export excludes a pending gesture. Sessions live in page memory; explicit file export is the persistence boundary.

`create`, `begin`, `update`, `commit`, `cancel`, `move`, `togglePin`, `undo`, `reset`, `exportDocument`, `importDocument` return new session values without mutating their inputs. Position updates are absolute model-space values; a gesture commits at most one undo item. `project` preserves defaults until edited, then expires routes only for moved endpoints. `overlaps` and `routeDiagnostics` report geometry conflicts; they do not solve them. No general arrange/router is implemented.

Only heading movement crosses the6CSS-pixel threshold. Actual drag pauses playback at retained progress; pointer cancellation, Escape, lost capture, blur and context switches restore its original position. Port/state click ownership stays separate. Arrow keys move10units, Shift1, and P pins. Exact source and independent browser evidence belong to[layout Task](../../.controller/domain-daemon/tasks/flow-atlas-layout.md) and[implementation Delivery](../../.controller/domain-daemon/flow-atlas-implementation.md#accepted-local-slice).
