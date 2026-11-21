---
schema: task
status: completed
description: Extract a minimal reusable declarative flow viewer core and verify it with an unrelated example.
links: [../runtime-atlas.md, ../../../.docs/architecture/runtime-atlas/README.md]
---

# Reusable flow atlas core

The user accepts the visible animation but needs readable pacing, complementary views over the same model, and a reusable tool for other systems. This Task owns the model/tool slice; the controller owns presentation/integration and actual checks. Use [AgentX-native orchestration](../orchestration.md) and no nested delegation.

## Ownership and outcome

Own only `tools/flow-atlas/{AGENTS.md,README.md,model.py,core.js,build.py,examples/request.json}` and this Task. Controller will move the existing four presentation leaves into `tools/flow-atlas/viewer/`, integrate their calls, adapt the runtime builder/model, generate HTML and execute tests/browser. Do not change existing atlas source/model/build/generated files, tests, root/shared guides, live services, dependencies or product files. Do not run builds/tests/browser/install/Cargo/product commands. Read root/module guidance, current `model.json`, `build.py`, `atlas.js` and actual user feedback before designing.

## Selected minimal contract

Keep the existing object/channel/mode/scenario/step/beat/initial+patch shape and `schemaVersion:1`. Add model-owned `factDefinitions:[{id,label}]`, `defaults:{modeId,scenarioId}` and optional per-mode `layout:{width,height,nodes:{[id]:{x,y}},routes:{[edgeId]:...}}`; do not introduce a graph editing platform, execution engine, plugin registry, migration framework or domain-specific lifecycle promises. A scenario is one authored ordered trace, not all branches or concurrent scheduling. `beats` are illustrative actions; final patches own facts. Unknown, retention and disconnect remain distinct. Node kind may include `actor|service` in addition to existing kinds; optional owner/executable/lifetime/modules/capabilities/sourceIds default to empty for unrelated systems. Source references may be empty; source paths are display/provenance data and generic validation must not require AgentX files.

`core.js` is a pure classic JS global `FlowAtlas` with `fold(model,scenarioId,index)`, `changes(model,scenarioId,index)` returning `{id,kind:'node'|'fact',label,before,after}`, and `defaultSelection(model)` returning `{modeId,scenarioId}`. No DOM/network/timers/AgentX IDs. Freeze these signatures early in the Task; notify controller of concrete necessary changes before implementing incompatible semantics.

`model.py::validate(model)` validates model structure/references/complete initial/patch/tones/facts/beat direction and visible endpoints. Clear ValueError with context; invalid input must fail before output overwrite. `build.py MODEL --output OUTPUT` uses shared `viewer/{index.template.html,atlas.js,atlas.css,layouts/workspace.css}` + `core.js`, embeds a safely escaped model in one offline HTML, and writes a receipt next to it. Expose an importable `render(model)` returning HTML text to let runtime builder keep its source-path checks and existing receipt. Generic build must work from unrelated cwd without `.agentx`, network, npm or Rust. Do not duplicate presentation sources.

Research reuse proportionately: compare this finite declarative trace requirement with adopting a diagram library or statechart/workflow engine. Record what the model expresses and explicitly does not express. Supply a small unrelated request→service→store/response example with different fact IDs and no AgentX identity. This example plus actual browser generation will establish reuse rather than a genericity claim based only on renaming files.

## Controller acceptance cases

Reject dangling/duplicate/hidden references, undeclared fact patches, wrong beat direction and unrelated edge. Preserve runtime original semantics. Build and open the unrelated example using the same viewer without code edits; inspect custom facts, generic topology, flow/sequence views and deterministic checkpoints. Verify speed changes do not alter trace facts. Controller retains actual evidence and closes the Delivery after integration.

## Specialist structural model and contract release

Bound specialist: `{serverId:srv_yDbm9zuqIMiM,agentId:21f5f935-f2da-40ae-bce4-302e2545315c}`. Workflow: inspect existing model and fold semantics → materialize native contracts → implement generic validation/build/example/docs → release source to the controller. No nested delegation, checks, generation or product/runtime operations belong to this assignment.

Dependency direction is consumer JSON → [model.validate](../../../tools/flow-atlas/model.py) → [build.render](../../../tools/flow-atlas/build.py) → embedded model + [pure core](../../../tools/flow-atlas/core.js) → controller-owned viewer. Python standard-library parsing/embedding and the existing SVG viewer are sufficient; external diagram libraries solve rendering but not the authored snapshot/diff/provenance contract, while statechart/workflow engines introduce branch/execution semantics this finite trace does not promise. Tool README will retain the comparison and authoritative external references.

Signatures are materialized in the owned sources. `validate(model)` returns a deep independent normalized copy, never mutating its caller. Required top-level additions are factDefinitions and defaults. Optional root sources/scope, node owner/executable/lifetime/modules/capabilities/sourceIds, edge transport/purpose/sourceIds, mode command/summary, scenario question, step note/certainty/sourceIds become empty strings/lists; missing patch nodes/facts become empty maps. Initial states remain complete in both mode and scenario; patches may update label/tone alone. Layout stays optional; present layout validates width/height, visible node keys and routes with start/end sides, optional ratios, via points and label coordinates. No generic source path is opened or required to exist.

Core signatures remain exactly `fold(model,scenarioId,index)`, `changes(model,scenarioId,index)` and `defaultSelection(model)`. Index -1 is initial; index i includes step i. Invalid/missing scenarios or out-of-range/noninteger indexes raise RangeError. Fold returns a new independent snapshot. Changes return node-label/fact strings in authored patch order; following the controller's compatible refinement, node entries also include beforeTone/afterTone so tone-only differences remain meaningful while preserving the original string fields. Declared defaults select one compatible mode/scenario; the core's empty/raw-model UI fallback uses first compatible objects or null, without application IDs.

Clarified validation boundary: direction is an explicit forward/reverse enum relative to an edge's declared endpoints; actor/handler describe the whole event, not every beat. Existing valid traces include downstream edges and reverse responses, so validation checks visible endpoints and beat membership in step.edgeIds without inferring causal direction from actor/handler. Mode channel lists may name observed context channels that do not carry a transfer in that step. These are preserved semantics, not execution claims.

## Source release and receiving acceptance

The specialist's bounded source assignment is completed and all owned writes are released. This is source delivery, not Delivery/browser acceptance. The [controller's multiview Task](runtime-atlas-multiview.md) receives executable generation, runtime adaptation, integration and actual evidence ownership. No viewer/model/runtime/generated HTML/test/product/service leaf was edited by this specialist, and no checks, builds, browser, package/install or product commands were run.

| Released artifact | Consumer result |
| --- | --- |
| [model.py](../../../tools/flow-atlas/model.py) | Contextual ValueError validation, deep-copy normalization, generic sources, complete states, partial patches, declared facts, safe IDs, beat direction/membership and visible layout references. |
| [core.js](../../../tools/flow-atlas/core.js) | Callable classic global with deterministic independent snapshots, changed values/tones and model-owned default selection. No DOM/network/time or application IDs. |
| [build.py](../../../tools/flow-atlas/build.py) | Importable render without output writes; standalone CLI and adjacent `.receipt.json`, safe model embedding and one-pass template slots. All sources resolve from tool location. |
| [request.json](../../../tools/flow-atlas/examples/request.json) | Unrelated fictional catalog: 3 nodes, 2 edges, 4 steps, 3 custom facts, reverse replies, partial/tone-only patches, no source/layout/application dependency. |
| [README](../../../tools/flow-atlas/README.md), [entry](../../../tools/flow-atlas/AGENTS.md) | Usage, schema/normalization/API and evidence boundaries, diagrams versus engine reuse decision, discoverable ownership routes. |

Static source reading inspected the public call paths, state/patch shape, example checkpoint evolution and build-before-write order. This is not executable verification. Proportionate reuse research read the official Mermaid sequence documentation and Stately machines page on 2026-11-10; the [tool README](../../../tools/flow-atlas/README.md#为什么保留这个小工具) retains source links and the decision. Existing template/UI sources remain controller-owned and singly located.

Recorded friction and disposition: hard-coded five fact labels/default IDs/layout belonged to presentation and prevented unrelated consumers; the selected model-owned facts/defaults/layout and generic fallback address that boundary. Actor/handler cannot validate every transport's direction because accepted downstream and reverse paths have different participants; explicit beat direction and declared visible edge membership are the preserved rule. Unsafe IDs could reach existing attribute selectors or object maps; the controller requested the small documented ID regex and prototype-key exclusion, now implemented. No additional approval or escaping framework is needed.

The controller can exercise the following concrete cases against this release, along with the original acceptance cases above:

| Case | Expected contract |
| --- | --- |
| From unrelated cwd, build the unchanged catalog example and open it using the same viewer. | Custom request/record/response facts, actor/service/store and generic topology; four checkpoints and reverse replies in all relevant projections. |
| Fold catalog checkpoints -1, 0, 1, 2, 3; mutate one returned status and fold again. | Stable independent snapshots; patch 1 changes archive tone alone, patch 2 restores it; final response is `200 · 条目 42`. |
| Compare changes at index 1; pass -2, 4, a fraction or missing scenario. | Archive diff includes stored→busy tones despite equal labels; invalid lookup/index raises RangeError. |
| Duplicate an ID/reference; hide a channel endpoint; use constructor or a quoted/bracketed ID. | Contextual ValueError before generation. |
| Remove an initial node/fact; patch an undeclared fact; use bad tone or local beat transport fields. | Contextual ValueError identifying the affected state/beat. |
| Give a beat direction `sideways`, or use `data-link` in request-sent whose only declared edge is `public-api`. | Reject malformed direction/unrelated beat edge; accepted downstream/reverse paths remain valid. |
| Add model text containing `</script>` and `__ATLAS_SCRIPT__`; attempt invalid input over an existing output. | Model text remains inert and round-trips; prior output remains intact on invalid input. |
| Compare renderer/build from unrelated cwd and change presentation speed/view. | Shared resource home and script order remain fixed; snapshot facts do not depend on time/view. |

No specialist process/service, listener, installation or scratch artifact was created. Existing listener18957 and prior generated artifacts remain under controller ownership. Remaining acceptance is explicitly the receiving controller's work, not an unfinished specialist implementation.

## Received executable acceptance

Controller independently accepted the released core/tool in [pacing and reuse outcome](runtime-atlas-multiview.md#accepted-pacing-views-and-reusable-outcome). Generic model/core checks, unrelated-cwd generation, safe embedding/output-preservation cases and actual offline three-view catalog browser use passed. The tool source is complete, this Task remains completed and the specialist is archived; historical source-only statements above describe that handoff boundary.
