---
schema: task
status: completed
description: Design a reusable typed domain/process visualization tool and its spatial interaction contract.
links: [../../../tools/flow-atlas/design/README.md, ./runtime-atlas-ports.md]
---

# Flow Atlas platform design

User requests a general-purpose object/domain/process/scenario modeling and visualization tool, with TypeScript contracts, SDK/oRPC reuse, draggable objects and readable connections. This assignment owns a concrete design and consumable TypeScript contract sketches, not a claim that the proposed platform is implemented. Retain the accepted v1 viewer, runtime listener and live user services.

## Outcome and method

Model the product before implementation: distinguish definitions from scenario instances, public methods from transport, authored explanations from observed evidence, process graphs from finite executions, and business state from presentation layout. Establish use cases, state/effect rules, public APIs, module ownership, technology choices and a complete first implementation slice. Use official oRPC/Standard Schema/React Flow/ELK/XState sources for selection; do not invent upstream behavior. Compile the bounded typed example and rejected misuse cases with existing tooling; no install, native/product build, Provider or service mutation.

Inline ownership is sufficient: this extends the already understood visualization domain and needs one connected design decision. Do not revive archived teams or spawn a review workflow. Preserve actual uncertainty in the design rather than resolving it through extra abstractions. The user has already authorized design; no confirmation gate is necessary.

## Working record

Current v1 semantics and exact acceptance: [canvas outcome](runtime-atlas-ports.md#accepted-canvas-port-and-state-outcome). Its model supports finite authored traces and one labelled state per node plus projected facts; it does not express type/instance separation, process branching or typed SDK import. Its rectilinear call paths do not avoid other objects and positions are fixed. These are capability gaps, not failures of the prior narrower evidence.

Research and retained source copies: `/tmp/flow-atlas-platform-design-kf3a92um`. One guessed oRPC Standard Schema URL returned404; use actual linked documentation rather than treating that URL as evidence. The official default docs additionally identify v2 beta, while npm stable was1.16.2; the design retains v1 contract/metadata evidence separately and does not infer v1 support from v2 integration docs. The v1 Standard Schema URL also returned404 and remains a recorded version/documentation gap.

## Completed design outcome

The[design entry](../../../tools/flow-atlas/design/README.md) now owns use cases, four-layer model, object/type/instance distinctions, domain/module/method/state contracts, process graphs and scenario visits, causal trace events, schema/evidence rules, module/API effects, layout interactions and implementation slices. Canonical design-only public declarations are[contracts.ts](../../../tools/flow-atlas/design/contracts.ts) and[api.ts](../../../tools/flow-atlas/design/api.ts); consumers are[order/reservation success and unknown](../../../tools/flow-atlas/design/example.ts) and[the tool's own drag responsibilities](../../../tools/flow-atlas/design/self-example.ts). Exact current v1 declarations remain untouched and discoverable alongside the proposed target.

Key refinements: Definition and instance trace are separate; one callId correlates request/return/outcome; emitted and received events have explicit ports and emission identity; state events alone own changed values; valid lifecycle edges are declarations, not a second reducer; Process expresses possible branches while Scenario visits witness one path; Layout is independent and keyed by actual content/scope. Unknown remote effects do not imply no effect, and later authority may resolve uncertainty without erasing history. Opaque internal business computation stays outside the tool.

Spatial contract covers pointer threshold/capture/cancel, keyboard/touch, drag-title versus Port clicks, pause-on-drag and retained playback progress, fixed nodes, undo, local draft/export, layout-only import, route-generation fencing and explicit unresolved lines. Route and arrange are separate APIs. React Flow and ELK are preferred mechanism candidates with explicit fixed-layout/obstacle probes; no dependency is installed and no unproven routing guarantee is accepted. The oRPC adapter reuses explicit runtime Schema/contracts and adds semantic bindings; it never runs handlers or fabricates process flow from DTOs.

Executed verification: `node_modules/.bin/tsc -p tools/flow-atlas/design/tsconfig.json` passed with existingTypeScript7.0.2, strict/noEmit and explicitES2022/DOM libs. This covers both positive consumers and five expected compile errors (invalid event, unknown-as-known value, disconnect-as-reply, invalid speed, state-as-position). A second pass was justified only by the final StateDifference/SchemaConversion signature refinements. No emitted files or extra toolchain. Typechecking checks structural contracts; it does not validate cross-ID semantics, run the future compiler, connect oRPC, test drag or exercise ELK. Those checks are assigned to the meaningful implementation slices in the design.

Design review against the user's request: the same types cover a business service and the tool's own modules; Source/Explanation attach to owned concepts rather than a standing tutorial; process branching, finite scenario causality, state changes, ports and channel lifetime retain distinct meaning. Native v1 compatibility is an explicit consumer acceptance condition, not a speculative migration subsystem. First implementation is a complete usable layout slice, then typed core/process views and an actual SDK/oRPC import; no micro-task tree or extra confirmation loop is created.

All relevant entries route to the target design: root/tool/design AGENTS, tool README/current contract note, Roadmap→Proposal→this design Delivery, and domain-daemon artifact catalog. Current runtime HTML/model/player/server, product sources, manifests/locks, `.bin`, `.agentx`, installed defaults and other services are unchanged. No subagents, browser or service were started. No source/resource claims remain. The detailed target remains proposed for discussion; completed here means design and contract/example work, not implementation acceptance.

Final bounded receipt and retained source copies: `/tmp/flow-atlas-platform-design-kf3a92um/final-design-receipt.json`. There is no pending work in this design Task; drag/routing/TS compiler integration remain concrete subsequent implementation outcomes.
