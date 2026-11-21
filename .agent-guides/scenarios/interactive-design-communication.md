---
schema: scenario
description: Explain complex domain models with a compact, componentized and panoramic HTML visualization
tags: [design, communication, visualization, feedback]
---

# Visual model communication

Use this method when the subject introduces a new model or has relationships, interacting states, multiple dimensions or spatial structure that people need to inspect. Simple content can be stated directly. Complex design content should be expressed as a visual HTML model, even when one view could also be drawn with Mermaid. This includes domain/data models, architecture, protocols, workflows and systems without a frontend. For an actual frontend page, also apply [continuous preview](../domains/webdevelopment/continuous-preview.md).

The HTML page exists to make the model visible: its objects, relationships, states, quantities and changes should be understandable from the screen. Build the visual model before adding controls. A page made mainly of buttons, select boxes, status paragraphs and sequential reveals does not satisfy this method.

## Model first

Recover the user's question, relevant scenario, canonical model or contract, accepted decisions and open alternatives. Keep those facts in their existing owner; the HTML page is a visual projection and exploration surface, not a second specification.

Abstract the domain before choosing a page structure. Identify the relevant objects, ownership, relationships, state spaces, constraints and dimensions of variation. Decide what must be visible together to answer the question, then map it to position, grouping, edges, lanes, axes, shape, line style, color and labels. Keep current behavior, proposed behavior, simulated actions and unverified assumptions distinct.

## Design the panoramic view

- **High density and compact layout:** give most of the screen to meaningful model information. Use concise labels, edge annotations, legends and local explanations. Avoid oversized headings, empty cards, repeated prose and control panels that displace the model. Keep text readable as density increases.
- **Visual states and dimensions:** show related objects, states, branches and comparison dimensions together wherever practical. Combine a relationship graph, state map, timeline, matrix, plot or spatial scene when each contributes a different view of the same model. Link their selection and highlighting.
- **One-screen orientation:** fit the decision-relevant overview to one desktop viewport where practical. Aggregate secondary detail or open a local detail view when the model is larger, but preserve a visible overview during focus, zoom and filtering. Do not shrink labels or force every field and combination into one screen.
- **Polished composition:** treat controls as part of the visual system. Use deliberate typography, spacing, iconography, hierarchy, affordances and selected, focus, disabled and loading states. Avoid browser-default buttons, raw select boxes, generic card grids and dense control strips. Use icons with accessible names and visible labels or tooltips where the meaning is not conventional.

Choose visual forms and maintained libraries that fit the model:

| Model question | Visual form | Interaction |
| --- | --- | --- |
| Objects, ownership and data | SVG or Mermaid relationship view with boundaries, labeled edges, cardinality and sample records | Select an object and highlight its neighborhood and linked details |
| States, lifecycle and rules | State map with guards, outcomes and relevant paths | Select a state or condition and highlight allowed and refused paths together |
| Protocols, asynchronous work and recovery | Participant lanes with message/effect timeline and failure branches | Scrub or select a message while retaining the full sequence |
| Architecture and alternatives | Boundary/dependency map with aligned traced flows | Highlight a dependency path or compare alternatives in the same frame |
| Quantities, algorithms and spatial behavior | Chart, plot, Canvas or Three.js scene with visible inputs, outputs and constraints | Adjust meaningful parameters and observe linked changes with a baseline |

Use Mermaid inside the page when it is a suitable visual surface; use SVG for selectable labeled geometry, Canvas for dense or continuously changing graphics, chart libraries for quantitative views, and Three.js or another 3D renderer when depth is part of the model. The page should coordinate these surfaces rather than become hand-written raw `div` markup. Reuse established project libraries and resources. Technology choice alone does not establish explanatory value.

## Deliver one componentized HTML file

Provide a named HTML file that the user can open directly. Put the model data, visual composition, styling and interaction in that artifact, using components or a small framework so the page can reuse professional visualization capabilities. Suitable approaches include:

- a single HTML file with browser ES modules and pinned CDN imports for React/ReactDOM, `htm` or precompiled components, Three.js, charting libraries and other needed packages;
- an existing Vite React or similar component project when the subject already has one; keep its real preview available and provide the generated or source entry according to the project contract;
- a self-contained bundled HTML file when the artifact must also work without network access.

Vite is a development and build tool, not a runtime that is embedded into a standalone HTML file. Do not describe a CDN React page as “embedded Vite”. Pin CDN versions and state the network dependency; bundle dependencies when direct offline opening is required. Do not build a general diagram editor, production persistence or a new application shell merely to discuss a model.

## Use interaction to explore the model

Interaction should act on the visible model: select, highlight, filter, compare, adjust a parameter, inspect a time position, zoom or manipulate a visual object. Coordinate linked views so the same object, state or effect remains identifiable across dimensions. Keep context visible when focusing and make hidden scope and restoration clear. Animation is optional explanatory support; provide pause, direct positioning and reduced-motion behavior. Sequential playback must never be the only way to discover structure or compare states.

Before expanding the page, inspect its initial view. The user should be able to identify the important objects, relationships and states without operating it, then use interaction to inspect what a static view cannot show. If the page mainly replaces the model with controls and changing paragraphs, redesign the visual composition and component styling.

## Share and update the view

The HTML file is the portable delivery artifact and may be opened directly. When the user needs a shared live view, serve that same file through an HTTP server reachable on the LAN and share its actual host address and path. A file path is valid delivery but does not claim shared browser access; `localhost` is not a LAN URL. Reuse the existing server and subject URL where possible.

Before sharing, check the actual page and its central interaction in a browser. Name the initial scenario, what the user can inspect and how to load updates. Keep the latest valid file available, record its source and run command in the existing work record, and preserve unrelated server instances. Prefer deliberate manual refresh; do not use unsolicited refreshes that interrupt exploration. If LAN access is unavailable, state that limit and provide the runnable HTML file.

## Work through the view

1. **Publish the overview.** Open a coherent model panorama early, with the relevant objects, relationships, states and branches visible together. Identify simulated or undecided parts.
2. **Explore in context.** Point to the relevant visual elements, vary meaningful conditions and use linked changes to discuss mechanisms and tradeoffs. Keep the overview available.
3. **Update the owners.** Record accepted meaning, alternatives and unresolved questions in the canonical model, design or contract. Update the visual projection and its data together.
4. **Consolidate.** Align the final model, page and claims. Keep run instructions, status, limits and unverified behavior with the subject. Keep the URL stable where possible and state when refresh is required.

This method adds no approval stage and authorizes no external messages or user-data changes. The responsible designer or implementer maintains the view with the affected model. [Design management](../domains/design-management/AGENTS.md) owns connected design maintenance; [observable feedback](observable-feedback.md) owns retained verification evidence. A visual page does not prove model correctness or actual service effects.

The portable [submission and recovery example](examples/design-discussion/README.md) retains an earlier button-led sequential simulation for its protocol semantics and server instructions. It is not a visual-method reference. A current version would show client knowledge, request identity and service effects across success, rejection and response-loss branches in one coordinated panorama.

The method is established when the user can understand the decision-relevant panorama, use purposeful interaction to inspect difficult relationships or variations, and see model changes reflected in the same portable page. A reachable file and working buttons alone do not establish explanatory value.
