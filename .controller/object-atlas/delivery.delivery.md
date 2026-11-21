---
schema: delivery
status: completed
description: Deliver a polished interactive SVG object workbench for understanding AgentX resources, attributes, methods and relationships.
stages:
  atlas:
    description: Design a coherent visual canvas, build a portable explorer and verify its real inspection paths.
    tasks: ["./tasks/atlas.task.md"]
---

# AgentX Object Atlas

The user wants a beautiful, precise object-oriented tool platform, not a static documentation diagram. A visitor should see the core object relationships without a tutorial, then select any visible object to understand its identity, properties, methods, state and neighbors. The canvas is the primary surface; the inspector, search and small state/workflow views are there to answer specific questions while preserving context. There are no live reads, mutations, or backend services.

The [native model](../../.docs/model/README.md), [new TS object model](../../.docs/model/typescript-domain-sdk.md), [current object contracts](../../packages/agentx-client/src/domain/contracts.ts), [HTTP panorama](../../.docs/api-panorama/README.md), and actual public client/native sources own the facts. The Atlas is a curated projection with source references, not a second canonical registry. Explain implementation status by shape/label as well as color: new TS object surface, callable HTTP/SDK resource, or native/context/presentation concept. Do not attribute methods simply because objects have a relationship, require no full generated DTO inventory, and do not claim pending integration has passed. The TS domain SDK effort proceeds independently.

Use [Canvas Design](../../.agent-guides/skills/canvas-design/SKILL.md) for an original visual philosophy and companion still, and [interactive model communication](../../.agent-guides/scenarios/interactive-design-communication.md) for the portable SVG/HTML workbench. The static-asset convention does not restrict the main deliverable: the user explicitly requested an interactive Web/HTML tool, with an SVG visual surface. The same visual system should serve the still and interaction. Apply [interface design](../../.agent-guides/scenarios/interface-design.md) and the user's lens: affordances and working content first; rationale in the handoff, no paragraphs of explanatory chrome.

The author owns visual hierarchy, original design, selection/filter behavior, and verified source-linked inventory. Controller kula owns scope, user evidence and final fit judgment. One specialist can compose the atlas without cross-author conflicts; no delegation cascade. Review the first coherent canvas in a real browser; fix material visual or semantic faults as a batch. Final acceptance requires both visible first-screen craft and operable exploration, not just source validity or click tests.

## Preview and artifact

Portable `index.html` and design materials under [the Atlas directory](../../.docs/object-atlas/). Serve only that directory at http://192.168.2.178:18987/ using `python3 -m http.server 18987 --bind 192.168.2.178 --directory .docs/object-atlas` from the repository root. Manual refresh; existing preview sites and AgentX are untouched. The opening will be shared once the canvas itself renders. The Task owns screenshots, exact operation and source-check evidence.

## Active assignment

AgentX launched `zali` with role `prototyper` on the [Atlas Task](tasks/atlas.task.md). The dedicated server is reachable and currently holds the authoring entry. Next: inspect the first complete canvas at natural scale and operate selection, sources and the receipt lens. Final evidence must describe the deployed static Atlas artifact, never production SDK or daemon adoption.

The first [interactive canvas](http://192.168.2.178:18987/) is now available. The [first 1440×900 capture](evidence/first-canvas.png) shows the selected Agent in the traceable core graph and the detail rail alongside it. Controller spot check opened Methods, selected Workspace, found Operation via the search keyboard submit, and entered the recovery view. The author owns final source review, narrow composition and full focused browser evidence.


## Accepted outcome

The [Object Atlas](http://192.168.2.178:18987/) is accepted as a portable visual workbench for the current model boundary. It exposes 21 inspectable objects, 68 source locations, three relationship views, global search, properties/methods/source tabs, and four receipt branches. The [portable HTML](../../.docs/object-atlas/index.html) and [static companion plate](../../.docs/object-atlas/companion.png) are source-linked; the final Chrome walkthrough covered desktop, touch-width mobile, direct offline opening, SVG keyboard activation, source copy, selection, reset and no-error/no-secondary-request behavior. The [Task evidence](tasks/atlas.task.md#evidence) owns exact screenshots and reproducibility.

The Atlas is a curated projection: its visual labels do not establish production support or live operations. The TypeScript domain SDK first slice is independently validated for its recorded boundary; true Workspace-filtered paging, broader SDK/domain coverage, user studies and live UI adoption remain separate.
