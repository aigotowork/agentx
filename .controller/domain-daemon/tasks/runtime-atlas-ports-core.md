---
schema: task
status: complete
description: Implement port/interaction model validation and pure projections with an unrelated outcome example.
links: [./runtime-atlas-ports.md, ../../../tools/flow-atlas/contracts.md]
---

# Port model and core assignment

Read root/tool AGENTS, current user intent and [canonical contract](../../../tools/flow-atlas/contracts.md). The human authorized object/port design and rapid implementation. Own only `tools/flow-atlas/{model.py,core.js,examples/request.json,README.md}` and this Task. Do not edit the design, viewer, builder, runtime data/docs, tests, generated output, root/shared context or any product/service file. No nested delegation, executable tests/build/generation/browser/formatter/install or product commands. Controller owns actual execution and integration, so source-only handoff is explicit.

Design and exact signatures are already fixed in the canonical contract. Materialize `FlowAtlas.calls(model,scenarioId,index)` and optional ports/interactions validation skeleton first, send an early usable signature/shape handoff, then implement and release coherently. Existing fold/diff/default semantics remain; existing data without ports stays valid. Use one cohesive validation helper rather than duplicating state fold logic across every branch. Record a concrete semantic gap directly rather than silently expanding schema. No new approval phase.

Extend the unrelated request example with actual command/query/event ports and success/rejected/unknown scenarios. Keep its existing lookup/read-record defaults,4 original step identities/patch values where practical,3 generic nodes and custom facts. Add authored response beats as needed so a declared response really returns along the request channel; never turn a disconnect into a response. Unknown outcome has no fabricated response and no successful change effects. Give examples explicit input and outcome contracts. Original source-free/no-layout generic reuse is retained.

Finish by updating tool README with schema/calls use and a link to canonical design. Release all source claims and notify controller. Actual generic negative cases, pure caller isolation, safe embedding, runtime semantics, offline generation and browser execution belong to controller, not this authoring Task.

## Source release

The bounded core specialist assigned by controller `7c3af077-454b-4d39-9c3d-71de3fbc9598` completed the five authorized leaves. All source claims are released to that controller. Canonical design remains [contracts.md](../../../tools/flow-atlas/contracts.md); [the receiving Task](runtime-atlas-ports.md) owns integration and actual execution. No shared design, viewer, builder, runtime data, test, generated artifact, product or service file was edited. No executable tests, build, generation, browser, formatter, product, install or service commands were run; there were no owned runtime resources to clean up.

| Artifact | Released behavior |
| --- | --- |
| [model.py](../../../tools/flow-atlas/model.py) | Optional ports/interactions normalization; operation/input/outcome references; visible owner/caller; real forward/reverse or local-self request; legal same-channel later reply; unknown-without-reply; changed patch effect references and rejected/unknown observation-only effects. One `_advance_state` helper folds each validated patch once for effect comparison, including tone-only changes. |
| [core.js](../../../tools/flow-atlas/core.js) | Exported `FlowAtlas.calls(model,scenarioId,index)` matches the canonical resolved shape. It resolves effects through existing `changes`, copies nested nodes/ports/inputs/results/effects and preserves authored order. Initial checkpoint returns `[]`; invalid scenario/index uses existing RangeError semantics. Existing plain copied maps and fold/changes/defaultSelection remain unchanged. |
| [request.json](../../../tools/flow-atlas/examples/request.json) | Three objects/two channels/three custom facts/five inbound ports. `lookup/read-record` defaults and the four original step IDs/patch values are retained. Success adds a fifth command step; `reject-selection` and `unknown-selection` each have one step. Same-step success/rejection replies return on `public-api`; unknown has a disconnect but no response or successful effects. Sources and layout remain absent. |
| [README.md](../../../tools/flow-atlas/README.md) | Documents optional schema, normalization, inert values, calls shape/isolation, checkpoint meaning and source-versus-execution boundary, with canonical design and receiving Task links. |

## Semantics and feedback

The existing read query spans several steps, whereas the selected `responseBeat` index is local to one step. Preserve that explicit limit: initial query ports declare receipt/processing acceptance; later reverse transfers invoke inbound observation event ports. They do not fabricate a cross-step response annotation or upgrade acceptance to completion. The appended command supplies a complete same-step request/reply example. This limit is recorded in the README; no schema expansion or competing reducer was introduced.

## Independent execution handoff

Source inspection matched references, effect differences and JSON structure. This is source review, not executed verification or feature acceptance. The controller should execute the affected existing suites and targeted cases once against the released source, then integrate the current viewer/runtime model.

| Case family | Positive and negative boundaries to execute |
| --- | --- |
| Compatibility/normalization | Legacy model omits ports/interactions; `{}` input accepted only with no required parameters; omitted effects/summary normalized; omitted responseBeat remains absent; no input mutation. |
| Ports/inputs | All three operation kinds; scoped outcome IDs; duplicate/unsafe port or parameter IDs; nonexistent owner/source; missing/extra parameters; nonboolean required; nested inert JSON copies without type coercion. |
| Transport | Forward, reverse and local-self invocation; hidden/mismatched caller or owner; disconnect/observe/retain as request; boolean/out-of-range indices; response before request, different channel, same direction, non-transfer or local-self response; unknown response rejection. |
| Results/effects | Declared outcome/summary; missing outcome reference; effect outside patch; unchanged label/tone/fact; real tone-only difference; rejected/unknown observation accepted and successful change rejected. |
| Pure projections | Initial `[]`, index RangeErrors, authored call/effect order, missing reply omission, resolved node/fact before/after and tones, mutations of nested projection inputs/port contracts/nodes unable to alter model or subsequent results. |
| Example/integration | Success final index is now 4; original indices 0..3 retain their prior patches. New scenarios each use index 0. Check all three separately, along with safe offline embedding, generic reuse and existing runtime models; browser stages must delay terminal results/effects until applied. |
