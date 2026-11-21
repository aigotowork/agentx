---
schema: scenario
description: Maintain layout-first semantic design systems with discoverable capabilities, owned tokens and independently designed switchable styles
tags: [design-system, layout, semantics, documentation, themes, maintenance]
---

# Design-system architecture and stewardship

Apply when creating, extending or reviewing a design system, Layout, shared
component, token set or visual style. For bounded work, reuse established decisions
and update only affected contracts and discovery paths. This method
composes the shared [design-system](./design-system.md)
and [component](./component-design.md) methods. When UIUX is adopted, read `uiux docs design-system` for its resource
formats, workbench APIs and discovery configuration. Otherwise apply these
maintenance responsibilities through the project's actual system and host.

## Own a system that people can find and extend

The system owner maintains Layouts, Components, reusable Blocks, semantic tokens,
styles, assets/effects, public entries, design rationale, usage and demonstrations
as one connected capability. Discoverability and explanation are part of delivery.
An implemented component with no reachable usage path is an incomplete addition.

Begin with the existing system entry, canonical source and actual consumers.
Locate the product's declared design/workbench entry and canonical package or module implementation owner. Bind these reusable methods to that consumer's actual artifact map. Preserve that ownership; directory examples in tool docs do not authorize
moving or duplicating canonical code. Keep source-to-consumer mappings discoverable.

## Design the Layout family first

Derive Layout needs from page relationships and tasks before designing individual
controls. Maintain enough reusable spatial patterns for the supported work:
for example, centered content, application shell, split workspace, list/detail,
inspector, toolbar/content and responsive grids. These are candidate responsibilities,
not a quota or a requirement to create unused Layouts.

For each adopted Layout, define named required/optional slots, alignment, spacing,
minimum/maximum constraints, container versus viewport sizing, overflow and scroll
ownership, responsive changes and nested composition. Include resize/collapse,
local return and overlay boundaries when applicable. Explain when to choose it
over neighboring Layouts; avoid a collection distinguished only by screenshots.

Implement space and containment in the Layout, interaction in Components, and
task presentation in Blocks. Do not give Layouts domain fetching or permissions.
Demonstrate actual controls and long content, empty/optional slots, constrained
containers and narrow arrangements. Design-time slot guides must not leak into
normal product UI. Follow `uiux docs design-system/components` for the full contract.

## Make names and structure express responsibility

Organize the catalogue by stable meanings: foundations/tokens, styles/themes,
Layouts, Components, reusable Blocks and assets/effects. Within each family,
group by capability when it improves discovery. Colocate subject implementation,
design and examples; split a subject directory when its responsibility warrants it.
Do not create empty folders or another parallel implementation for each style.

Use semantic directory names, symbols, slots, variants, fields, callbacks, CSS
classes and variables. Prefer `inspector`, `selected`, `onSelectionChange` and
`surface-raised` to positional names, unexplained flags and palette literals.
Define each public field's meaning, units where relevant, optional/absent meaning,
allowed values and event intent in its native types and adjacent usage guidance.
A request callback and a confirmed result must not share misleading names.

Keep native types authoritative; documentation links them rather than maintaining
a second prop schema. Expose supported public entries independent of stories,
fixtures and workbench internals. Keep private parts private and shared dependencies
pointing toward their foundations. Follow the actual host's file naming conventions
and `uiux docs design-system/structure` rather than inventing a resource format.

## Maintain a traceable variable system

Use the chain foundation values → semantic roles → component/Layout roles →
rendered consumers. All authored presentation choices use named variables/tokens,
including local geometry. Concrete numeric/color values belong at their owned
definitions; page assembly must not scatter magic numbers or fixed inline styles.
Keep asset path geometry and domain quantities at their real owner rather than
turning every number into a global token. Runtime measurements use an explicit
validated binding boundary.

Maintain the meaning, type/unit, default or reference, valid range where applicable,
scope, supported overrides, consumers and reset behavior of variable groups and
editable settings. Explain why a scale or role exists and which changes it permits.
Use the native token format's available fields and adjacent design documentation;
do not require a duplicate hand-maintained table for every primitive value.

Choose one authority for equivalent CSS/TS/native values. Identify derived outputs
and their refresh path; preserve aliases and reference relationships. Add a local
semantic role for a real local need, promote it when compatible shared consumers
exist, and record rename/deprecation migration when removing a public role.
Follow `uiux docs design-system/tokens` for validation and source editing.

## Design each visual style deliberately

Keep brightness, visual style and density as independent supported axes. A light
pixel interface and a dark pixel interface share the same style responsibility;
they are not separate component systems. A new palette alone does not establish
a new visual language. Read `uiux docs design-system/visual-language` before a
new direction or material style extension.

Every supported style must have its own reachable `<style>.design.md` or a
style-owned `design.md`, linked from the system's design entry and catalogue.
Use the existing subject naming convention. Shared principles may be linked,
but each style must explicitly state its own rationale and distinctive decisions.
An experimental style also records its intent and incomplete support honestly.

The style design must explain:

- Intended domain, audience, desired impression and purpose; concepts, metaphors,
  references and why this language fits the task.
- Visual grammar in concrete design terms: hue/chroma/value and contrast,
  typography and hierarchy, spacing/rhythm, silhouette and geometry, material,
  stroke, depth, texture, icon/illustration language and motion character.
- Chosen tradeoffs, rejected directions when useful, use/avoid cases and limits
  for readability, density, accessibility and runtime cost.
- How the grammar maps to semantic tokens, renderer/asset strategies and actual
  Layouts, Components, Blocks and states; link their canonical implementations.
- Rules for adding compatible resources and components, supported combinations
  and representative scenes that demonstrate the language in use.

For a pixel style, design the grid, stepped silhouettes, stroke rhythm, discrete
depth and actual icon/illustration family. Applying pixelated rendering to smooth
artwork or removing corner radii alone does not meet that intent.
Every added Block/component should inherit this rationale, or document a scoped
extension at its owner and update the shared design if its principle changes.

## Make style switching a supported capability

Expose declared style choices through the existing system Provider/controls and
the actual consumer's appearance boundary. Share component identities, props and
state meanings across styles. Map semantic variables and asset roles; use a scoped
renderer strategy where geometry cannot be expressed safely through variables.
Keep bright/dark choices independent of style and density.

Declare supported combinations, defaults, inheritance/override behavior and any
unsupported combination explicitly. Switching must preserve selection, drafts,
focus, business data and the other appearance axes. Nested boundaries and portal
content must receive the appropriate appearance context without leaking into
the host. If settings are persisted/imported, validate supported values and define
fallback/reset without overwriting valid current state on an invalid import.

Demonstrate switching, returning to the original style, nested override/inheritance,
affected brightness combinations, overlays, meaningful states and narrow content
in composed scenes. A select control or token map alone does not establish that
multiple styles are implemented. A new system should establish at least two
complete agreed style treatments to demonstrate switching; in a bounded extension,
identify the existing baseline and add only the requested style. Record missing
style coverage in an existing system rather than claiming it already supports it.

## Keep documentation progressively discoverable

Maintain this linked path in the existing documentation/catalogue structure:

`system entry → capability family → subject usage/design → native types and runnable examples`

The entry explains the system purpose, design principles, source ownership and
supported appearance choices. Family indexes expose available responsibilities,
not just a list of filenames. For each public subject make its name, responsibility,
use/avoid guidance, public import, design/usage, example and support/maturity
reachable. Native resource manifests or CSF navigation can supply the catalogue;
do not duplicate the same inventory in several Markdown lists.

Subject usage includes a minimal real import/composition, important props and
state semantics, accessibility/container constraints, style support and links to
related alternatives. Explain Layout slots and token overrides where relevant.
Keep long details at the subject; the top entry should help the reader choose.
Private helper files need no individual catalogue entry, but their public owner
must be clear. Experimental and deprecated items must be distinguishable.

For new React catalogues use the installed tool's standard Storybook/CSF discovery
route (`uiux docs storybook`); retain native resources where already adopted.
Do not create duplicate resource/design/scenes registration for an existing CSF
catalogue. Style rationale remains a linked design document in either format.
Additions, moves and removals update the owning index, public entry, usage and
examples in the same change; preserve stable IDs and migration links where needed.

## Extend and finish coherently

Before adding, locate the nearest existing capability and its style rationale.
Choose an existing variant, local composition or new resource according to meaning.
Avoid unrestricted styling props, a growing set of unrelated Boolean flags or
style-specific forks. Grow coverage from real consumer needs; maintain common
controls deliberately without inventing a universal component backlog.

Author the complete declared system slice in broad passes, then check affected
contracts, source/discovery links, real imports and composed behavior. Walk from
the documentation entry as a new consumer: choose a Layout, find a Component,
understand its fields and use its public example. For appearance changes, operate
the switching cases above; use `uiux check` only when native manifests/references
change and relevant type/build checks when public code changes.

Finish when the affected capabilities are findable, explained, operable and
extensible through one canonical source, and observed limitations are explicit.
Maintain design, source, index and examples together throughout later changes.
This is part of the existing design-system responsibility, not a separate review
role, mandatory new skill, approval step or document set.
