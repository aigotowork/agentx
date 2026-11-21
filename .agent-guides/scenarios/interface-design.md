---
schema: scenario
description: Required interface design constraints for products, pages, management consoles, tools and mobile screens
tags: [product-design, ui, layout, information-density, design-system]
---

# Product and interface design requirements

Read before designing, implementing or reviewing a product interface, page, prototype, management console, tool or mobile/small-screen experience. These are required constraints, not optional aesthetic suggestions. Apply them to the affected scope of a bounded change without restarting completed product discovery. Product work without a user interface does not require this guide.

## Trust users; make the interface explain itself

Use structure, visual hierarchy, familiar controls and direct feedback to communicate purpose. Do not patronize users with paragraphs explaining obvious controls, repeated card descriptions or instructional banners. Such text consumes the space needed for useful information and actions, reducing the user's overall control of the task.

For compact actions, use this preference: **icon + tooltip > text button > explanatory text**. Keep icons consistent and give every control an accessible name. Make tooltip information reachable by keyboard and touch; hover cannot be the only access on a small screen. Use a concise text label when an icon would make the action ambiguous. Keep necessary field names, units, consequences and error/recovery guidance at the point of use; they do not justify surrounding the interface with prose.

## Make resource relationships visible

Before choosing panels or cards, identify the user's core resources, their hierarchy and relationships, and the main action chain. Make that structure visible in the workspace. A work-management product, for example, should connect the selected outcome, its tasks, participants, outputs and acceptance state through actual supported relationships. A visual grouping does not establish new storage facts or capabilities.

Choose trees, tables, grouped rows, relationship views or another form that serves the actual decisions. Related resources must remain distinguishable and connected without explanatory paragraphs. A single step's question or tutorial must not displace the surrounding working resources. Open detailed questions, conversations, evidence and forms locally while preserving selection and context. Keep task goals, questions and findings as working data; they are not explanatory clutter.

## Design layout first; require information efficiency

Identify the user's scenarios, their relationships, the primary task, the facts needed together and the available actions. Then establish the layout skeleton: named regions, their hierarchy, space allocation, persistent controls, scrolling and responsive behavior. Implement functional Blocks and Components inside that structure. Do not build isolated feature cards and hope their final stack forms a layout.

For tools and management workspaces, fit the core task's information and almost all related interactions into one viewport workspace, including secondary and tertiary actions reached through local disclosure. This requirement is especially important on mobile and other small screens. Keep the user's attention on the working subject; oversized controls, empty cards and low-information sections must not displace useful context.

For example, a small-screen English-learning workspace must accommodate the text, audio player, playback modes, word lookup and progress together. Keep text in its main region and playback/progress available; put occasional options in a top-corner popover and secondary/tertiary playback controls inside the player. A long text may scroll inside its region while the working controls and context remain available. Do not distribute this one task across a series of oversized sections or unrelated pages.

Evaluate density using the same task, representative data and viewport: record the useful facts visible together, the number of usable actions directly available, and the secondary/tertiary actions reachable within that workspace. Distinguish these levels and the steps needed to reach them. A long scrolling route does not count as one viewport; hidden, unreachable or unrelated controls do not increase useful density. Keep text legible and controls operable rather than shrinking them to inflate a count.

## Derive disclosure from scenario and information hierarchy

Determine which scenarios share a subject and working context, which are subordinate, and which are substantially different. Keep related facts and actions together. Use the lightest local layer that preserves focus:

| Need | Surface |
| --- | --- |
| A compact control's name or brief clarification | Tooltip, available on focus and touch as well as hover |
| Secondary or tertiary options | Popover, menu or nested control group |
| Supporting content or occasional detail | Collapsible region or drawer |
| A bounded confirmation or focused input | Modal |
| Selection, progress or availability | Consistent visual state through color, size, emphasis and an accessible status cue |
| A substantially different task or context | A separate page or Screen with a clear return path |

Minimize visual jumps and route changes within one task. Do not default to separate pages merely because the viewport is narrow. Preserve the subject, selection, draft, progress and focus across disclosure and return. Color and hover support meaning; they cannot be its only carrier.

## Keep the visual language coherent and visual expression primary

Tools and management products must be visually consistent, concise, clear and focused on the working subject. Prefer visual expression to explanatory prose: use alignment, grouping, icons, state, progress, charts and relationship views to make information immediately readable. Visualizations are a core design method for management and tool products; select meaningful representations for their actual data and decisions. Decorative charts do not improve information efficiency. Prefer light palettes unless the established product direction or an explicit user requirement calls for another theme.

Reuse a coherent component foundation. Depending on the product, suitable choices include shadcn/ui with Lucide, TanStack Table, React Hook Form and Zod; shadcn-admin or Next.js admin templates can supply reference patterns. Ant Design can be more suitable for some management products. These are reuse options, not a requirement to install every library. Use charts where they communicate comparisons, trends, distributions or relationships more efficiently than prose, retaining access to exact values when needed.

## Make composition semantic at every level

Encapsulate almost every meaningful region as a named unit that owns its capability and visual treatment. Use this composition model, omitting only levels with no distinct responsibility:

```text
Page/Screen = Layout + Blocks/Sections
Block/Section = Layout + Components
Leaf Component = primitives + semantic HTML + CSS classes
```

Assembly layers must express domain meaning through Layouts, Blocks, Components, variants and data types. Do not fill them with anonymous HTML trees, styling utilities or literal presentation values. Layout internals own spatial arrangement; leaf components own low-level markup and styling. Native semantic HTML belongs inside those implementations.

A component boundary does not require a separate `.tsx` file. Keep simple private components with their Block; split out complex or substantially reused components. In React, define private component functions at module scope and pass their inputs through props, so rendering the parent does not recreate component types and reset child state. Use the existing [interface structure](../domains/webdevelopment/interface-structure.md) for ownership and [React boundaries](../domains/webdevelopment/implementation.md#react-component-boundaries) for implementation.

## Require semantic classes and variables for styling

All authored styling must be expressed through CSS classes with meaningful roles. Concrete presentation values must come from named CSS variables/design tokens; raw values belong in their authoritative token definitions. Do not add magic numbers, fixed inline styles, arbitrary-value utility classes or scattered literal colors, sizes and spacing. Tailwind-style utilities are suitable when backed by the declared token system and encapsulated in the owning Layout or Component; they do not make anonymous assembly markup semantic.

Give local presentation needs scoped semantic variables and classes rather than exceptions to token use. Keep tokens at their actual shared or local owner; do not promote every local role into a global catalogue. Semantic naming applies equally to layout regions, Blocks, Components, states and data types. The assembly layer consumes those meanings rather than implementing their private styling. Use the [design system](../domains/webdevelopment/design-system.md#token-and-theme-design) for the token/theme structure.

Before calling the affected design complete, inspect the layout with representative content at its target size and operate the primary and disclosed actions. Check information/action density, local continuity, visual consistency, semantic composition and token-based styling. Use existing design/task notes for consequential choices and evidence; do not create a separate document set. Static source or a wireframe alone does not establish the rendered experience.

## Review resource workspaces

Review the affected design at a stated viewport with representative data and a concrete task. Keep observations in the existing design or Task.

| Criterion | Evidence | Substantive failure |
| --- | --- | --- |
| Core resource structure | Locate the work hierarchy, related participants, outputs and acceptance references from the workspace | One step's story hides the resources or their supported relationships |
| Useful information density | Compare distinct useful objects/fields at the same viewport and comparable data; count each fact once | Explanations, oversized containers or duplicated summaries displace necessary facts |
| Action density and reachability | Identify immediate actions and the steps to reach secondary actions from their objects | Resources have no usable next action, or related work needs unrelated route changes |
| Visual meaning | Inspect grouping, hierarchy, alignment, icons and state without instructional prose | Removing prose leaves no intelligible resource model; deleting sentences alone does not repair it |
| Context and interaction | Follow detail, edit, result and return, including keyboard and narrow-screen use where applicable | Selection/drafts are lost, compact actions are inaccessible or sparse sections obstruct the task |
| Truth and scope | Distinguish reported facts, unknowns, delivery and acceptance through concise local cues | Mock capability is presented as implemented, or compactness hides unknowns, errors or effects |

Do not use arbitrary row/action quotas, dummy records or decorative metrics to pass a review. If structure or useful density fails, reconsider layout and interaction before polishing wording. A collision-free screenshot is insufficient; a static SVG supports visual assessment only. Correct substantive gaps within the owning work; this guide adds no review committee or approval gate for minor polish.
