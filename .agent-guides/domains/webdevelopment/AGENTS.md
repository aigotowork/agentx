---
schema: document
description: Progressive entry point for web product design, frontend implementation and experience review
---

# Web Development

Use this entry when designing, building, changing or reviewing a web product, page, prototype or component. Follow the [parent guidance](../../AGENTS.md) and select all relevant rows below; do not read this directory as a sequential manual. A narrow fix can go directly to its owning section. These are responsibilities that one agent can perform, not mandatory specialist roles or authority to delegate.

Before interface design, implementation or experience review, read [product and interface design requirements](../../scenarios/interface-design.md), including for a bounded fix, tool, management page or mobile screen. They are required constraints; a direct route to a narrow section does not bypass them. Reuse this guide if already read.

For every frontend page design and implementation, apply [continuous preview](continuous-preview.md) before substantial private work: establish a reachable LAN preview early, keep the page current throughout, and prefer stable use with deliberate manual refresh. For new model or complex solution discussion, also apply the shared [visual model communication](../../scenarios/interactive-design-communication.md) method.

## Working model

Treat frontend development as the composition of user tasks, interface responsibilities, behavior and visual expression. A collection of controls is not a product experience; a theme is not a complete design system; functioning code alone does not establish a usable interface.

- **Structure:** an application composes Pages, Pages compose Screens or smaller units, and each level uses child contracts while owning its own model. Keep private child complexity below that boundary. A Frame owns layout; these are responsibility distinctions, not mandatory nesting levels or React types.
- **Component contract:** treat public component interaction and page flows as protocols. For non-trivial units, establish data sources, state ownership, events and transitions before detailed visual design; then map behavior to structure, variants, actions and feedback. Include relevant motion, assets, responsiveness and accessibility; inherit shared rules.
- **Shared system:** tokens, primitives and patterns provide coherence. Shared design resources have a product-level home; Pages and business components own scoped presentation resources and task-specific design.
- **User perspective:** examine what a user can understand and accomplish in a concrete visible state, then follow the action into the next state.
- **Proportionality:** reuse established decisions; investigate missing capabilities early; produce only the artifacts and checks that make the current work understandable and verifiable.

## Select by task

| Situation | Read/apply |
| --- | --- |
| Designing or refining interface hierarchy, state feedback, microinteractions, visual assets or motion | [Interaction and visual craft](interaction-visual-craft.md), composed with interface requirements and the affected motion/asset methods |
| Maintaining Layout families, semantic APIs/tokens, switchable styles or progressive system documentation | [Design-system architecture and stewardship](design-system-stewardship.md), with the existing system and component methods |
| Reviewing a tool/editor/workbench across complete tasks, object scope, precision or interruption/return | [Tool experience review](../product-design/scenarios/tool-experience-review.md), with [tool interfaces](tool-interfaces.md); use [interaction modeling](../interaction-modeling/AGENTS.md) for scenario/action/call coverage |
| Starting a product, feature, redesign or work spanning several frontend concerns; coordinating authorized work | [Workflow](./workflow.md): scope, capability gaps, dependencies and the shortest useful delivery path |
| Defining user goals, navigation, information hierarchy, content, task flows or a prototype's experience | [Experience design](./experience-design.md) |
| Structuring a management console or workbench across multiple subjects, relations and actions | [Management product design](../management-product-design/AGENTS.md): product-to-page derivation, complete workflow and implementation/acceptance; select Web realization methods here as needed |
| Choosing a destination, overlay, deep link, return path, focus transition or cross-route recovery | [Navigation and recovery](navigation-and-recovery.md): surface, address, draft/effect lifetime and responsive continuity |
| Creating, changing or reviewing page/screen structure, layouts, component boundaries or frontend directories | Required before this work: [Interface structure](./interface-structure.md), including layout-first composition, dedicated layout ownership and framework mapping |
| Designing an empty feature or a parent whose required children do not yet exist | [Reuse discovery](./interface-structure.md#reuse-before-new-boundaries), then [contract-first composition](./interface-structure.md#compose-before-implementing-internals): design with explicit placeholders and implement children inward |
| Managing or exploring project `uiux/` resources with the UIUX tool | [UIUX tool support](./design-system.md#uiux-tool-support): read tool-owned requirements and contracts through `uiux docs`, then use its check/dev/build commands as needed |
| Establishing or changing visual direction, tokens, themes, shared UI behavior or branded component variants | [Design system](./design-system.md) |
| Designing or refining component/Screen treatments, comparing alternatives, maintaining state examples or collecting interactive design feedback | [Storybook design workbench](./design-system.storybook.md): reference-led design, operable stories, scoped experiments, user feedback and consolidation into canonical components |
| Organizing design-system documents, tokens, SVG/artwork, animation resources or subject-local presentation collections | [Design-resource structure](./design-system.md#design-system-artifacts-and-structure): shared product home, local scope, documentation boundaries and source-to-consumer ownership |
| Designing, reusing, customizing or changing a component's public behavior and appearance | [Component design](./component-design.md) |
| Designing or reviewing parent/child interactions, frontend domain interfaces or page operation flows | [Component and page protocols](./interaction-and-state.md#component-and-page-protocols): local inputs/actions/state are protocols too; maintain focused diagrams and working conclusions |
| Integrating an HTTP/RPC backend, checking API schemas or maintaining a generated frontend client | [API contracts and SDKs](../../scenarios/api-contracts-and-sdks.md): confirm the maintained protocol/data definitions and use a reproducible consumer toolchain |
| Discovering reusable UI, adopting or publishing a registry, or making a design system usable by agents | [Component discovery and registries](./registry.md): connect available items, current local contracts and rendered examples |
| Designing meaningful component states/events or changing asynchronous behavior, URL/global state, APIs, forms or recovery | [Behavior before presentation](./interaction-and-state.md#model-behavior-before-presentation), then the relevant interaction/state sections and targeted general modeling links |
| Adding or changing transitions, animation, gesture feedback or animated media | [Motion](./motion.md) |
| Preparing or integrating images, illustrations, icons, fonts, video, sample content or other visual resources | [Assets](./assets.md); combine with motion for animated assets |
| Implementing, debugging or integrating a web interface; running its development environment | [Implementation](./implementation.md), selecting the sections relevant to the change |
| Modifying `.ts` or `.tsx`, including non-web modules | [TypeScript and React contracts](./implementation.md#typescript-and-react-contracts); no other web sections are required solely by the file extension |
| Observing browser state, component/source facts or retained runtime/visual evidence | [Browser observation](observation.md): the appropriate scene, original identity, capture limits and independent effects |
| Designing an inspector, run reader, data/configuration browser or workbench | [Tool interfaces](tool-interfaces.md): scope, overview, object relations, evidence and action/recovery |
| Modeling content or preparing visual candidate batches and component handoff | [Content and assets](content-and-assets.md): meaningful content, explicit render briefs, source families and preparation/generation boundaries |
| Assessing usability, visual quality or a completed flow | [Experience review](./review.md#experience-review), then the applicable evidence section |
| Reviewing a component, shared component base or design-system adoption | [Component and system review](./review.md#component-and-system-review); add experience review for affected user journeys |

Rows compose. For example, a mobile editor may require experience, structure and state work; a hero with custom illustration may require design-system, asset and motion work. Reuse context already read. Re-select when a new concern appears.

## Maintaining this guidance

Keep frontend scenario methods here. Reference general [engineering](../../scenarios/engineering-design.md), [state modeling](../../scenarios/data-state-modeling.md), [code structure](../../code-structure.md) and [documentation](../../documentation.md) at the relevant boundary instead of duplicating them. Independent Skills retain their own packages; a scenario selects a Skill when its capability is needed and available.

When changing this system, consult the owning product’s design and task records when changing its source. Preserve the distinction between structural levels, professional concerns and execution scenarios. Add a file only for an independently useful reading need, and update both incoming routes and affected cross-references.
