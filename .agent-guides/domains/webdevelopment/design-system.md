---
schema: scenario
description: Establish visual direction, tokens, themes, design-resource homes and an interactive component-design workbench
tags: [web, ui, design-system]
---

# UI Design System

Use this guide when establishing or changing visual language, tokens, themes, shared interaction conventions or reusable component styling. Every UI project should adopt or establish an intentional foundation before detailed visual assembly. Reuse an adequate existing system; a small change does not require redesigning it.

First read [product and interface design requirements](../../scenarios/interface-design.md). Their visual language and semantic CSS/variable constraints are required throughout shared and local styling; reuse this context when already read.

## What the system owns

The system defines shared UI conventions within a declared scope. Give its design resources a discoverable home using the [shared/local structure](#design-system-artifacts-and-structure); the responsibility list does not require one folder per row.

| Responsibility | Owns | Typical source |
| --- | --- | --- |
| Product expression | Audience, visual intent, tone, density, hierarchy and interaction principles | A concise design direction with representative references |
| Foundations | Color, typography, spacing, sizing, radius, borders, elevation, icon language and responsive rules | Canonical token definitions |
| Semantic theme | Meaningful roles and state variants independent of a particular palette | Theme definitions or CSS variables |
| Primitives | Shared accessible behavior, visual roles and deliberate variants | Project-owned component contracts and implementations |
| Shared patterns | Repeated interaction, feedback and composition conventions | Pattern contracts and representative usages |
| Motion and asset language | Shared timing, expressive tendencies, illustration/icon style and resource constraints | References to [motion](./motion.md) and [asset](./assets.md) decisions |
| Interactive design workbench | A shared place to explore, operate and evaluate components and compositions | [Storybook workflow](./design-system.storybook.md), canonical implementations and maintained state examples |

Actual routes, Page layouts, domain workflows and business-component decisions belong to their [local owners](./interface-structure.md#local-design-descriptions). Adopting this system does not remove the need to design those compositions.

## Choose a visual direction

Translate the product's users and tasks into visible choices: emphasis, typographic hierarchy, information density, spacing rhythm, surface treatment, contrast, iconography, imagery and motion character. “Modern,” “premium” or “use this library” is not a sufficiently concrete direction.

Use a small reference set when useful, identifying the transferable qualities rather than treating one screenshot as a universal template. Distinguish product constraints from taste. For uncertain high-impact choices, compare a few representative treatments; do not generate a mood board or multiple variants when the project already has a clear direction.

Compose a representative scene with realistic content and important states. Check whether visual emphasis supports the primary task, whether typography and spacing create a coherent rhythm, and whether custom assets or specialized components are needed. A palette alone cannot answer these questions.

## Establish the minimum coherent foundation

1. State the visual and interaction intent.
2. Identify the foundation values and semantic roles needed by the representative scene.
3. Define applicable themes and interactive states, including focus, selected, disabled, invalid and pending behavior.
4. Select or retain the primitive base using [component reuse](./component-design.md#reuse-and-adaptation).
5. Adapt actual primitives and reusable patterns to the tokens and intended behavior.
6. Compose the scene, expose gaps, and revise shared contracts or retain local exceptions according to ownership.

This is an iterative foundation, not a prerequisite to design an entire component catalogue. A small product can begin with a few tokens and primitives while keeping the extension path clear.

## Token and theme design

Use a layered token model when it supports the product:

```text
foundation value -> semantic role -> component/state usage
palette accent   -> primary action -> primary button background
neutral value    -> foreground     -> text and meaningful icons
spacing value    -> control gap    -> form and toolbar rhythm
```

Foundation tokens describe values; semantic tokens describe purpose. Components must consume semantic roles through CSS classes. Concrete presentation values belong in named variables/token definitions, including local values; a local need is not an exception permitting magic numbers or scattered literals. Add scoped semantic variables when existing roles cannot express the requirement, without enlarging the global token set for every local layout choice.

Choose one canonical source for each token set and identify generated outputs. Record units, theme overrides and meaningful state variants. Themes should override semantic roles rather than make each component branch on palette names.

Consider color and contrast together with typography, spacing, density, focus, feedback and surface layering. Prefer light palettes for tools and management products unless an established product direction or explicit user requirement calls for another theme. Supported themes must remain coherent across actual components and assets; changing a root variable file is not sufficient evidence. Define supported variants instead of promising arbitrary themes.

## Express the system in components

Customize both primitives and business components intentionally. Express all authored styling through semantic CSS classes, with concrete values supplied by variables/tokens. Keep shared expression in tokens, variants and patterns, and scoped classes with their actual owners. Do not use fixed inline styles, arbitrary-value utilities or literal presentation values in assembly code. Token-backed utilities can implement Layouts and leaf Components; higher assembly layers consume named components and semantic variants.

Choose variants by meaning and supported behavior. Keep loading, selected, destructive and invalid states consistent with their actual semantics. Do not expose style escape hatches that bypass semantic classes and variables. Local extensions use scoped semantic roles and supported variants under the same constraints.

Apply shared interaction conventions to focus restoration, feedback placement, modal versus inline editing and other repeated patterns. The [component contract](./component-design.md#the-complete-component-contract) owns the local behavior; the [state guide](./interaction-and-state.md) owns transitions and effect boundaries.

Define shared motion tendencies here and keep effect-specific trigger, timing, interruption and reduced-motion decisions in [motion contracts](./motion.md#motion-contract). Define a coherent asset family while keeping each consuming scene's resource requirements with its [asset brief](./assets.md#resource-brief).

## UIUX tool support

Use the `uiux` tool when available to manage and explore a project's `uiux/` resources. Start with `uiux docs design-system` for its design requirements, then `uiux docs` to select relevant topics; `uiux docs design-system/structure` owns the tool-specific directory and resource contracts, and `uiux docs usage` explains setup and commands. Keep these tool contracts in the tool's maintained documentation rather than copying them here.

Run `uiux check <project>` to validate configured resources, `uiux dev <project>` for the interactive workbench, and `uiux build <project>` when a static workbench is needed; build replaces the generated `uiux/dist/` directory. Use `uiux init <project>` only for a project without an existing `uiux/` directory. Subject-local `uiux/` collections express ownership; the tool loads configured project systems, so nesting alone does not register resources. Reuse an established Storybook installation when appropriate; adopting UIUX does not require a parallel workbench.

## Interactive design workbench

Use [Storybook as the design-system workbench](./design-system.storybook.md) for substantial primitive, pattern or Screen design. Translate principles, specified styles and references into visible choices, then evaluate behavior, visual treatments, motion and resources in interactive stories. Let users operate concrete scenarios and tie their feedback to the scene and revision under discussion.

Distinguish modeled states, supported variants and temporary design candidates. The workbench helps resolve design choices and retain accepted examples; canonical components and resources remain the implementation consumed by the product. A prototype can answer a design question while service integration or other required states remain unfinished. Apply the detailed workflow only to the affected design, preserving established decisions and proportionate checks.

## Design-system artifacts and structure

Use a product/project-level `uiux/` as the default home for shared presentation rules and resources. Keep it at the actual product owner; a multi-product repository need not have one universal root system. Preserve an established equivalent such as `src/uiux/` or a shared package when it fits the build and ownership model; do not maintain another copy solely to match this name.

### Shared and local homes

| Material | Natural home | Boundary |
| --- | --- | --- |
| Shared visual direction, UI/UX conventions, tokens/themes, icon/illustration/font families and motion definitions | Product `uiux/` or its existing equivalent | Own shared presentation meaning, reusable resources and their usage guidance |
| Storybook configuration, shared preview context and system showcases | `uiux/storybook/` or its existing equivalent | Own the workbench; discover subject-local stories and import canonical component/resources without a parallel implementation |
| Page/Screen/Block/Component-specific visual composition, token aliases, artwork or animation resources | `<subject>/uiux/` when a resource collection is useful | A scoped extension of the shared system, not an independent theme fork; one small stylesheet can remain beside its component |
| Feature goals, domain/interface/state design, architecture and business workflow | The existing `.docs/` or subject-adjacent spec and source contracts | Link presentation decisions/resources rather than moving domain rules into the design system |
| Component behavior and runtime lifecycle | The owning component/module implementation | Keep state authority, events, service calls and effect cleanup with the executable owner |

Shared interaction conventions belong with the design system; the specific user's task flow and legal state transitions belong with the Page/component model. A visual or motion specification references that model instead of maintaining another behavioral specification. Shared primitives can remain in their existing canonical component homes, linked from the system entry; do not centralize domain components merely because they have styling.

### Resource layout

An illustrative layout, not a scaffold to create in full:

```text
<product>/
  .docs/designs/                   # feature, architecture and protocol decisions
  uiux/
    README.md                      # scope, base system and consumption entry points
    guidelines/                    # visual language and shared interaction conventions
    foundations/                   # canonical tokens, themes and shared motion values
    assets/                        # reusable visual families; group by meaningful owner
    motion/                        # shared effect specs/presets and animation resources
    storybook/                     # shared design workbench; see its focused guide
  pages/Checkout/
    Checkout.tsx
    Checkout.interface.ts
    Checkout.spec.md                # behavior; links to presentation design
    uiux/
      Checkout.visual.md
      Checkout.tokens.css          # local semantic roles referencing shared tokens
      confirmation/                # local artwork/animation brief, sources and exports
```

Create only useful groups; resource families/effects can keep their design notes, editable sources and delivery files together. Choose token/style formats supported by the project. Local `uiux/` collections may occur at any meaningful owner, including nested Components, without a copy of the shared foundation or a mandatory folder at every level.

Reference shared semantic tokens and motion conventions; namespace justified local extensions and scope their styles. Make token/theme imports and asset loading explicit: directory nesting does not implement runtime inheritance. Shared resources must not depend on a Page's private files. Promote a resource to the nearest meaningful shared owner only when compatible real consumers need it; update references and retain one canonical source.

### Source-to-consumer ownership

Keep the resource path traceable: **authoritative source → optional transform/export → runtime import or served asset → consuming scene**. Editable SVG/artwork, animation definitions and token data may be consumed directly or produce derived outputs. Identify which files are authored and how any exported/generated files are refreshed; do not hand-maintain equivalent source and delivery copies. Runtime `public/`, build or package-output locations follow the existing toolchain and do not become a second design authority.

Use the system's existing entry and, where adopted, its [Module catalogue](../../documentation.md#module-artifacts) to expose scope, inherited rules, canonical resource locations and usage/examples. Keep one catalogue; a README or local AGENTS.md explains how to use it without duplicating the inventory. A local entry links its shared base and owning component/spec. Keep resource gaps and live progress in the existing task rather than copying task status into the catalogue.

During design and implementation, locate the shared foundation and relevant local resources before adding new assets or values. Connect chosen resources to the component contract, then check the actual scene in relevant states/themes, viewport conditions and reduced-motion behavior. Retain the ownership decision, source-to-consumer links and unresolved gaps in the [working record](../../scenarios/project-context.md#make-method-execution-explicit); use [assets](./assets.md) and [motion](./motion.md) for their detailed methods.

## Make the system usable by agents

Expose an entry into actual available capabilities, their intended use and representative rendered states. A token file and a written visual direction alone do not tell an agent which task compositions already exist. Keep the current local inventory aligned with source contracts and accepted [Storybook examples](./design-system.storybook.md), with experiments clearly distinguished; use [registry discovery and distribution](./registry.md) when consuming external items or sharing the system across projects.

Package stable decisions after they exist in working components. A registry can distribute a base, themes, components and supporting conventions; usage contracts explain appropriate composition, and examples demonstrate the result. Follow the registry guide's staged adoption path instead of building a catalogue service before there is a real consumer.

Validate changes using [component and system review](./review.md#component-and-system-review): render representative components in affected themes and states, check meaningful exceptions, and inspect at least one composed consumer. Review motion and assets in that same context; isolated token values cannot establish visual coherence.
