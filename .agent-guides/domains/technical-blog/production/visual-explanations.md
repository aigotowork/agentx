---
schema: document
description: Explain structure, time, state, quantities and evolution with purposeful, interpretable model views
---

# Visual explanations

Use when a reader must reason about relationships that a visual can expose. A figure is a model with scope and semantics. Select its question before its notation; verify its actual renderer separately through [publication checks](../publication/renderer-and-release.md).

## Assign each view a question

| Reader question | Suitable view | Semantics to preserve |
| --- | --- | --- |
| Who interacts with which system? | C4 context | People, system boundary and external parties |
| Where do execution and data live? | C4 container | Applications/stores, responsibilities and communication; container does not mean Docker |
| Which parts collaborate inside one executable unit? | C4 component | One selected container, internal responsibilities and ports |
| What is derived from what? | Generation/build flow | Source, transformation and artifact; arrows mean derivation |
| What happens during an operation? | Sequence | Participants, ordering, guards, effects and consequential rejection/failure |
| Which transitions are legal? | State view with conditions | State meaning, trigger, guard, durable change and retry outcome |
| Who owns an identity or relationship? | Relationship view or typed table | Identity, cardinality, lifecycle and authority |
| How can an evolution preserve service? | Before/after views and staged sequence | Mixed-version window, transition conditions and recovery boundary |
| How do quantities differ? | Plot or comparison table | Units, population, source, scale and uncertainty |

C4 supplies abstraction boundaries; Mermaid supplies notation. A folder tree is not a container view and an import graph is not a runtime sequence. Use explicit names for a view and its relation type. Separate derivation, invocation, data flow and deployment unless a clear legend resolves the distinction.

## Reveal one model through successive questions

Orient, follow a path, then challenge it. Reuse stable names so each view adds a distinction instead of introducing a competing model. A complex explanation often needs structure, dynamic behavior and failure/state views; choose them from actual questions rather than a diagram quota.

For example, a shared-operation article can first show a contract deriving descriptions and binding a handler. That supports reduced description duplication. A second view traces admission, durable write and external effect, including rejection and post-commit failure. That reveals which facts survive. A third view can propose durable effect intent, attempt ownership and retry states, clearly labeled as a proposal. The first diagram cannot establish authorization; the third cannot establish that a queue exists today.

The [contended-resource example](../examples/contended-resource.md) shows how a structural question, event trace and transition rule answer different parts of one argument.

## Make every figure interpretable

Introduce its reader question. State the scope, omitted details, edge/encoding meanings and exceptional styling close to it. Follow it with the observation and its consequence. A figure should not need a preceding page of prose to decode its identities.

Use domain labels and verbs on important relationships. Group by responsibility, process or trust boundary only when that boundary matters. Drawing two stores inside one box does not establish a shared transaction. A request arrow alone does not say whether the result is durable, acknowledged or merely scheduled.

Allocate space for refusal and partial failure. A success-only path can erase the mechanism the article is explaining. Split a large view at a meaningful boundary and connect it with stable names. Choose a useful narrow-screen order; if labels require constant page-wide zoom, simplify, split or provide an accessible larger view and nearby text path.

Use color redundantly with labels, line style or shape. Explain the important relationships and conclusion in text; image alt text can be too short for a complex model. Preserve identity and direction across prose, controls, diagrams and code. Interactive views also follow [experiment acceptance](interactive-experiments.md#accept-the-model-and-the-experience).

## Deliver Mermaid and exported figures honestly

- Use syntax supported by the selected Mermaid version. Ordinary flowcharts can express accurately labeled C4 views; specialized C4 syntax is optional.
- Give nodes stable identifiers and quote labels containing parser-sensitive punctuation. Keep exact rules in canonical sources; diagrams project them.
- Parse/render with the actual shipping version. Check labels, edges, light/dark contrast and narrow screens. Valid-looking source does not establish a rendered figure.
- Retain editable source and its relation to exported SVG/PNG. Place the export in the consumer's actual public media path; a design note does not publish it.
- Honor the host's trust boundary for diagram settings, links, embedded HTML and SVG. A diagram feature must not silently grant CMS authors arbitrary script execution.
- If rendering is unavailable, choose a verified static export or name the required renderer implementation. A Mermaid fence shown as code does not meet a promised diagram deliverable.

## Encode quantities and space accurately

Use quantities only with the [measurement basis](../planning/research-and-evidence.md#measurements-and-comparisons). Select encoding from the comparison: line for change over ordered time, bars/dots for categories, distribution views for spread, scatter for association and composition views for meaningful parts of a defined whole. A flow with quantities can justify Sankey; a generic process cannot. Association alone establishes no causal direction.

Use comparable scales in aligned comparisons, readable units and an adjacent data summary. Bars normally need a zero baseline because length carries magnitude; any other encoding or restricted axis needs an honest, visible rationale. Show uncertainty where the data support it. Label estimated or synthetic values near the figure. Do not invent benchmarks for a decorative dashboard.

Prefer 2D for topology and state. Justify 3D through a meaningful spatial property or third variable. Perspective and occlusion often obscure dependency relationships; rotation alone does not solve that problem. If 3D serves the task, explain the dimension and provide a 2D/table route to the important conclusion.

Finish when the needed views preserve the model, expose the decisive case and meet the actual rendering/access requirements. More diagrams are unnecessary when they repeat the same observation.
