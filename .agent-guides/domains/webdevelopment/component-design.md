---
schema: scenario
description: Define complete component contracts and choose, adapt and maintain reusable UI capabilities
tags: [web, components, design]
---

# Component Design

Use this guide for a new or materially changed component, reusable pattern, business Block or specialized view. The [structure guide](./interface-structure.md) determines its responsibility; this guide makes the boundary sufficient for design, implementation and use.

## Choose the kind of capability

| Kind | Responsibility | Design emphasis |
| --- | --- | --- |
| Primitive | A broadly reusable control, surface or feedback mechanism | Accessible behavior, tokens, states and deliberate variants |
| Shared pattern | A recurring interaction or information composition | Consistent task behavior, slots and child relationships |
| Specialized component | A focused capability such as a chart, editor or media viewer | Domain of interaction, data shape, runtime needs and extension limits |
| Business Block | A cohesive small domain task composed from other components | Business meaning, state ownership, capability calls and user outcomes |
| Page, Screen or Frame | Higher-level composition responsibilities | Use the [responsibility model](./interface-structure.md#responsibility-model) before detailing internals |

These distinctions do not impose separate base classes or folder taxonomies. Several kinds may use the same React mechanism. Visual complexity and domain responsibility are different dimensions.

## Start with behavior

For a non-trivial component at any level, first define its data sources and authority, meaningful states, incoming events, transitions and effects using the [frontend state model](./interaction-and-state.md#model-behavior-before-presentation). Then map that behavior to visible structure, variants, actions and feedback. Keep the model with the component's contract/spec; simple input-to-view components need no explicit machine.

Treat the public collaboration as a [component/page protocol](./interaction-and-state.md#component-and-page-protocols). Use its focused diagrams to design and review inputs, actions, state confirmation and page flows; retain local behavior behind the boundary instead of exposing every private state.

## The complete component contract

Describe the relevant facets before implementing complex internals. Use existing source declarations and local design context; a trivial component needs no separate specification. Inherited rules can be linked, and an inapplicable facet needs no invented behavior.

| Facet | Decide | Keep authoritative in |
| --- | --- | --- |
| Purpose and scope | User task, responsibility, intended consumers and exclusions | Public usage contract; local rationale only when needed |
| Inputs and actions | Values, identifiers, callbacks/commands, event meanings and result/failure semantics | Types and public declarations |
| State and effects | Data sources and authority, controlled/internal/derived state, events, transitions and external effects | State owner and capability contracts |
| Structure and composition | Regions, slots/children, parent/child roles, content sizing and scroll ownership | Named composition and local design |
| Visual expression | Map modeled state and current data to structure, tokens, variants, themes and feedback | Shared system and [scoped presentation resources](./design-system.md#shared-and-local-homes), linked to the local state-to-view mapping |
| Interaction and accessibility | Keyboard/pointer/touch behavior, semantics, focus and feedback | Behavior implementation and applicable pattern contract |
| Motion | Meaningful transition, trigger, interruption and reduced-motion behavior | [Motion contract](./motion.md#motion-contract), if needed |
| Assets and content | Required resources, placement/crop, text and data shape, fallbacks | [Resource brief](./assets.md#resource-brief), if needed |
| Adaptation and runtime | Narrow layouts, localization, server/client constraints and relevant performance limits | Local design and integration boundary |
| Evidence | Important states and representative consumers that demonstrate the contract | Existing examples, fixtures, stories or targeted checks |

The facets are a design lens, not a mandate for ten files, props for every styling detail or a second hand-maintained copy of the TypeScript API.

For substantial primitive, pattern or Screen design, use the [Storybook workbench](./design-system.storybook.md) to turn the contract and visual direction into operable state examples. Explore palette, hierarchy, assets and motion against the actual scene; keep temporary candidates distinct from supported variants and carry the selected result into the canonical component. The workbench method belongs to the design system; this component still owns its behavior and local design.

## Design the public boundary

Expose the smallest useful contract that preserves the consumer's intent. Prefer semantic variants and explicit state models over unrelated Boolean flags that allow impossible combinations. Name callbacks for the event or request they represent; distinguish a request to save from confirmed persistence.

Choose controlled or internal state deliberately and describe synchronization or reset behavior when relevant. Do not duplicate authoritative values locally merely to make JSX convenient. Keep private DOM, library hooks, calculations and effects behind the boundary.

Use slots or children for meaningful composition, not to avoid deciding the component's responsibility. Avoid passing application-wide state objects or transport clients through a primitive's interface. A business Block can coordinate domain capabilities through an established contract; it should not force callers to know its private fetching or rendering details.

Review visual and behavioral decisions together. For example, a selectable result is not fully specified by `selected: boolean`: selection semantics, keyboard behavior, pending feedback, contrast, animation and long-content layout may matter to its actual use.

## Reuse and adaptation

Start with the user's required capability and [scoped reuse discovery](./interface-structure.md#reuse-before-new-boundaries), then choose the smallest compatible solution:

1. Reuse a suitable project component or pattern.
2. Compose installed primitives or platform capabilities.
3. Investigate a specialized component, richer block, headless behavior, visual effect or asset provider for a demonstrated gap.
4. Build the remaining product-specific behavior or expression.

Use [registries](./registry.md) for external discovery and distribution. For a remaining gap, a [contract placeholder](./interface-structure.md#compose-before-implementing-internals) lets parent design proceed before the child's internal implementation.

Do not stop at basic controls when the task needs a richer composition. Also do not add several overlapping UI systems merely because their demos look attractive. Candidate classes solve different problems:

| Candidate class | Verify before adoption |
| --- | --- |
| Headless behavior primitives | Interaction semantics, focus, keyboard behavior and responsibility for all styling |
| Styled component base | Theme/provider model, supported variants and compatibility with the product's visual system |
| Copy-owned or registry components/blocks | Source quality, provenance, imported dependencies, update ownership and adaptation effort |
| Specialized interactive component | Actual data/interaction contract, accessibility, extensibility and runtime cost |
| Motion or visual-effect component | Specific visual purpose, token alignment, interruption and reduced-motion alternatives |

For a consequential choice, use [technology selection](../../scenarios/engineering-design.md#technology-selection-design): verify current official documentation, actual APIs, versions, license, maintenance and compatibility with the project. Include styling precedence, server rendering, bundle/loading cost and replacement cost where relevant. Reuse an accepted choice when its constraints are unchanged.

When working with shadcn/ui, use the available [shadcn Skill](../../skills/shadcn/SKILL.md) for registry, composition and customization procedures. A Skill supplies technical execution methods; the product still needs the design and ownership decisions above. Do not treat the conversation's library lists, component counts or proposed stack as validated requirements.

## Integrate an adopted component

Inspect the source and public behavior before copying or installing. Map tokens and variants to the local system; identify introduced styles, providers, packages and imports. Preserve accessible interaction while adapting appearance. Remove irrelevant demo content and wire real callback semantics.

Choose a shared or local owner according to actual meaning and consumers. Record source/version or revision, relevant license obligations and deliberate deviations where needed for maintenance. Copied source becomes code the project must understand and maintain; avoid updates that overwrite intentional product behavior without review.

Exercise the important states in context: populated and long content, empty/pending/failure where applicable, keyboard/focus, narrow layouts, affected themes and reduced motion. A polished demo is selection evidence, not acceptance evidence for the product.

## Evolve without fragmenting the system

A new need may belong in a semantic token, existing variant, shared pattern, specialized component or local business view. Put it at the lowest layer that owns its meaning. Similar screenshots alone do not justify a shared abstraction, and a local exception does not automatically justify a fork.

When public behavior changes, inspect actual consumers and compatibility before implementing. Use [component and system review](./review.md#component-and-system-review) for duplicated contracts, divergent variants, hidden dependencies and composed behavior. Keep design rationale separate from the [TypeScript usage contract](./implementation.md#typescript-and-react-contracts).
