---
schema: scenario
description: Scope frontend work, identify missing capabilities and deliver coherent user outcomes
tags: [web, workflow, design]
---

# Frontend Workflow

Use this guide for a new product, feature or redesign, or when coordinating work across frontend concerns. A local change within an established contract needs only the corresponding path below. Use the [entry](./AGENTS.md) to select detailed methods.

When the user problem, product scope or cross-domain design handoffs remain uncertain, use [product design](../../scenarios/product-design.md) to connect needs, prototypes, contracts and feedback. Continue here for frontend delivery; a bounded frontend change does not require a new product discovery cycle.

Establish the relevant professional methods and their expected artifacts before implementation. The paths below express dependencies, not serial task batches: prepare independent structure, state, resource and contract work concurrently. Keep the workflow current when evidence changes its assumptions. Test execution and image reading follow the [shared limits](../../principles.md#quality-tests-and-images).

## Establish the outcome

Start from the user's task and the existing product. Identify the intended audience, entry point, successful outcome, affected journey and important constraints: devices, input methods, accessibility, content, visual ambition, available data and delivery scope. Distinguish an exploratory prototype from a production implementation.

Batch the relevant product/page context, design-system and component/external-contract reads. Use representative populated and failure-state models; read a rendered image only for UI design or user-intent review and explain its meaning. Do not infer the entire experience from an empty screenshot, a component library or a technology name.

Resolve consequential ambiguity using existing evidence, a focused question or a small prototype. State assumptions that affect the result. Do not turn every unspecified visual detail into a blocking question.

## Choose a proportionate path

| Work | Useful sequence | Completion surface |
| --- | --- | --- |
| New product or major feature | User task and regions → scoped reuse and system foundation → composition contracts and resource needs → representative slice → expand and review | A coherent reachable journey, with important states and shared rules demonstrated |
| New page in an established product | Inspect neighboring journeys and available local capabilities → composition skeleton → implement missing contracts and resources | The page works in its entry, exit and return contexts |
| Structural design before child implementation | Required capabilities → reuse/adapt/gap decisions → named child contracts and placeholders → parent composition | A reviewable structure with explicit unimplemented boundaries; child internals can be implemented or handed off later |
| Visual redesign | Preserve required behavior → choose visual direction → adapt tokens, variants and assets → render key states | Consistent design across the affected states, themes and viewports |
| Interactive prototype | Define the design question → realistic sample states and navigable skeleton → inspect and revise | The prototype answers the question; simulated data, actions and limitations are explicit |
| Interactive component or Screen design | Design intent and state contract → [Storybook workbench](./design-system.storybook.md) → compare relevant treatments → user feedback → consolidate | Reviewable scenarios and selected design are linked to canonical components; remaining simulations and integration are explicit |
| Component addition or repair | Inspect actual callers and contract → reuse/adapt or design the gap → implement → exercise affected states | Public usage and composed behavior remain coherent |
| State or integration bug | Reproduce a transition → locate the state/effect owner → correct behavior → verify the failing path and relevant adjacent transition | The original failure is resolved without losing context or recovery |
| Asset or motion task | Read the consuming contract → prepare the brief → produce/adapt → inspect in context | Accepted resources work in the actual layout and behavior |
| Review-only request | Select outcomes and representative scenes → gather evidence → identify causes and owners | Prioritized findings with reproduction context and proportionate remedies |

Do not finish an isolated component catalogue when the requested outcome is a complete journey. Conversely, a narrow component repair does not require a new product plan or design-system rebuild.

## Discover missing capabilities early

For affected scenes, consider the following before detailed implementation. Record only consequential gaps in the existing design or task record.

| Concern | Question that exposes missing work |
| --- | --- |
| Experience | What must users understand, decide and do? What happens before and after this scene? |
| Structure and behavior | Which Page, Screen or Block owns the task, state and external action? |
| Visual system | Can existing tokens, primitives and variants express the intended hierarchy and identity? |
| Composition and reuse | Is a richer pattern or specialized component needed beyond basic controls? |
| Motion | Which transitions require continuity, feedback or a deliberate expressive effect? |
| Assets and content | Which actual images, icons, fonts, illustrations, examples or copy does the layout need? |
| Integration and runtime | Which contracts, data states, devices or delivery constraints could invalidate the design? |

Classify a gap as covered, requiring a decision, or requiring a deliverable. “Use shadcn” does not settle visual direction, advanced composition, animation or asset needs. Reuse established choices; research only the demonstrated gap through [component selection](./component-design.md#reuse-and-adaptation) or the relevant capability guide. Use [registry discovery](./registry.md#agent-consumption-loop) when searching reusable items, while checking the current local contract before importing an already installed capability.

## Keep the work visible to the user

Apply [continuous preview](continuous-preview.md) to every frontend design and implementation. Make the first meaningful slice reachable on the LAN before expanding substantial implementation, share the URL, and keep it current at useful intermediate states. For structural or complex model discussion, use [visual model communication](../../scenarios/interactive-design-communication.md); a componentized HTML visualization can communicate an unimplemented boundary without pretending the production behavior exists.

## Build an observable slice

For implementation or prototype work, make a representative user task operable early, with credible content and explicit fixtures where needed. Use it to inspect hierarchy, layout, boundaries and resource needs together. Structural-design-only work can stop at the [named composition and contract skeleton](./interface-structure.md#compose-before-implementing-internals); do not force missing child implementation into that stage.

Keep [experience](./experience-design.md) and [structure](./interface-structure.md) models small but explicit. For non-trivial [component contracts](./component-design.md), establish [data sources, states/events and transitions](./interaction-and-state.md#model-behavior-before-presentation) before detailed presentation or complex internals. Read or establish the [design system](./design-system.md) to map behavior into visual expression, and prepare [assets](./assets.md) and [motion](./motion.md) requirements while the design still permits adjustment.

Review at the level of change using [review](./review.md). A user-goal or navigation failure returns to experience design; a repeated visual inconsistency returns to the design system; a local behavior failure returns to its component or state owner. Do not patch every symptom with more CSS or global state.

## Coordinate by contract when authorized

These concerns can be handled by one agent. If collaboration is authorized, use the project's [delegation criteria](../../roles/controller.md#when-to-delegate) and [handoff guidance](../../scenarios/task-handoff.md). Divide work by bounded deliverables with stable interfaces, not by job titles alone.

A handoff identifies the consuming scene/component, user purpose, inherited design rules, inputs and outputs, relevant states, constraints, artifact location and acceptance evidence. An asset or motion specialist also needs the actual [resource brief](./assets.md#resource-brief) or [motion contract](./motion.md#motion-contract). The integrator checks the composed result; a specialist's completion does not establish that the product journey is complete.

Keep decisions with their owners and execution state in the existing task record, following [placement rules](../../documentation.md#artifact-placement-rules). Link contracts, designs, previews and evidence; do not copy the same specification into each assignment.
