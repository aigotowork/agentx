---
schema: scenario
description: Compose applications, Pages, Screens, Frames, business Blocks and Components with explicit ownership
tags: [web, architecture, composition]
---

# Interface Structure

Read this guide before creating, changing or reviewing page/screen structure, layouts, component boundaries or frontend directory organization. Select the relevant pattern below; use [experience design](./experience-design.md) for the user task and [component design](./component-design.md) for public contracts.

First read [product and interface design requirements](../../scenarios/interface-design.md), including their viewport efficiency and semantic assembly constraints. Reuse this context when already read; a direct structure route still requires it.

## Responsibility model

These names describe responsibilities, not mandatory framework types or directory levels. A project may call the same complete task view a Page or a Screen; either can represent one screen of content, including scrolling content. Use the project/framework vocabulary consistently. Distinguish a route entry from its inner views only when they have different responsibilities; do not create both Page and Screen wrappers merely to satisfy these names. In React, all these units can be components.

| Concept | Owns | Boundary cue |
| --- | --- | --- |
| Application/site | Page relationships, cross-page navigation, shared shell, system-wide modes and composition of shared services | Several user tasks need a coherent product |
| Page | A stable entry into a user task or domain context, commonly associated with a route; composes major regions and coordinates the use case | Users can enter, revisit or navigate to the task as a whole |
| Screen | A coherent task view, either the page itself or an inner work context with its own visible state and actions | A project calls its task views Screens, or a Page has distinct working views such as list, detail or an editing step |
| Frame/layout | Spatial regions, slots, sizing, responsive arrangement and scroll boundaries | Layout should change independently of the tasks placed inside it |
| Business Block | A cohesive small domain task, its UI composition and the local state/actions required by that task | A page region has behavior and a meaningful contract beyond presentation grouping |
| Component | A reusable or local unit of presentation and behavior behind a public contract | A responsibility is independently understandable and changeable |

Composition is recursive: products compose Pages, Pages compose Screens or smaller units, Screens compose Blocks/Components, and Blocks compose smaller Components. Each level owns its own task model and coordinates children through their public contracts. A parent needs the child's capabilities, inputs, events and observable guarantees; the child's private rendering, state machinery and dependencies stay below the boundary.

Page/Screen assembly must read as Layout + named Blocks/Sections; Block assembly must read as Layout + named Components. Encapsulate meaningful regions and their visual treatment. Keep anonymous HTML trees and styling details inside the owning Layout or leaf Component, not in the assembly layer. Private component boundaries do not require separate files; use the [React boundaries](implementation.md#react-component-boundaries) when realizing them in React.

Skip levels that add no useful responsibility. Frames arrange content at several levels, and the design system supplies shared rules across them. Complexity or screen size can suggest a boundary, but the task and contract determine it.

“Block” in a vendor catalogue may mean only a visual section or copied template. Classify the adopted code by its actual responsibility; a hero layout is not automatically a business Block. A Screen is neither a synonym for every viewport size nor a requirement for a separate URL.

## Establish layout before content

For a Page/Screen, Block or other unit with meaningful spatial regions, first identify those regions and reuse or define a layout component. Establish its skeleton before filling it with business content. Keep layout visible in a dedicated `layouts/` directory under the nearest owner; an established `frames/` directory is an equivalent convention. Do not bury structural components among unrelated business components or leave the whole region arrangement inline in the page.

- Layouts own region placement, spacing, sizing, responsive arrangement and scrolling. Keep business data fetching, domain rules and task state in the owning feature/Block.
- Express content through `children` and named slots such as `header`, `sidebar` and `footer`. Prefer composition to combinations of boolean configuration switches.
- Keep a layout used by one screen under that screen's `layouts/`; use feature-local `layouts/` for a shared domain structure and a shared `src/layouts/` or design-system layout scope for genuinely common structures. Promote only when compatible consumers exist.
- A simple leaf component needs no separate layout component or directory. Create the directory with an actual layout implementation, not empty scaffolding. Existing layouts should be reused before adding new ones.
- Use semantic HTML where the region warrants it: `header`, `main`, `aside`, `nav` and `footer`. Give repeated landmarks distinct accessible names with `aria-label` or `aria-labelledby`. Nested layouts must not each introduce a `main` landmark; semantics follow the composed document.

For example, a screen composes business Blocks through a layout imported from its owning `layouts/` directory:

```tsx
function OrderScreen() {
  return (
    <OrderLayout header={<OrderHeaderBlock />} sidebar={<OrderFiltersBlock />}>
      <OrderListBlock />
    </OrderLayout>
  );
}
```

The layout imports no order Blocks; it receives content through slots. A useful structural review can find the layout owner, read the named regions from the composition and change their arrangement without moving business logic into the layout.

## Reuse before new boundaries

Discover capabilities through the directory map before inventing components. Read descriptions and public contracts first; open implementation details when compatibility or the intended edit requires them.

| Search scope | Action |
| --- | --- |
| Current Page/Screen/feature | Inspect owned Components, Blocks, examples and existing compositions |
| Enclosing and shared owners | Follow directory descriptions/catalogues to compatible domain, layout and design-system contracts; cross domains only through public entries |
| Missing capability | Search selected external [registries](./registry.md#agent-consumption-loop) or other suitable existing solutions |
| Remaining gap | Define a new local contract; keep the reason for reuse/adaptation/new work in the existing composition note when consequential |

Search by task and behavior, not only by component name or appearance. Avoid a whole-repository inventory when a scoped map suffices. A new product may have no local components, but can still reuse an adopted foundation and external patterns.

## Compose before implementing internals

Use **task/regions → reuse discovery → child contracts → composition skeleton → child implementation → composed verification**. Apply the same pattern recursively at Page, Screen, Block or Component scope.

Name the required capabilities and define the minimal props, events, state ownership and visible constraints before implementing missing children. Reuse actual contracts when they exist. Apply [behavior modeling](./interaction-and-state.md#model-behavior-before-presentation) to non-trivial contracts, exposing only the states and events relevant to the parent. Keep the parent focused on arrangement and orchestration; private child transitions, calculations, markup and effects stay with the child owner.

| Design need | Suitable placeholder | Boundary |
| --- | --- | --- |
| Review structure and contracts without running the scene | A named composition model, typed signatures or an explicit fail-fast stub | Mark the unimplemented capability; do not treat a compiling skeleton as working behavior |
| Inspect layout or navigate a prototype | A visible placeholder or fixture-backed component using the intended contract | Keep simulations explicit; an unhandled throwing child must not prevent the required preview |
| Implement or hand off a child | Its public contract, local spec and parent composition as the reference | Implement within the boundary; propose contract changes when required rather than leaking internals upward |

Remove required placeholders before claiming the implemented journey complete; an explicitly scoped prototype can retain declared simulations. When authorized delegation is useful, use the [contract handoff](./workflow.md#coordinate-by-contract-when-authorized); otherwise implement inward in the same workflow.

References and screenshots inform hierarchy, density, objects and visible actions; unseen behavior remains an assumption to resolve. Use [general contracts](../../scenarios/engineering-design.md#contract-boundaries-and-integration) at domain/API boundaries.

## Let source structure express the model

Apply [ownership and colocation](../../code-structure.md#ownership-and-colocation). Nest a capability under its actual owner and promote it only when real, compatible consumers require broader scope. Directory descriptions and public entries form the local discovery map.

| Usage scope | Natural owner |
| --- | --- |
| Screen used by one Page | That Page's local Screen directory |
| Component/Block used by one Screen | That Screen's local component directory |
| Component/Block used by several Screens of one Page | The Page's shared component scope |
| Capability reused across Pages in one domain | The domain/feature's public scope |
| Product-wide primitive, Frame or stable common capability | The design-system, shell or other meaningful shared owner |

Local `screens/` or `components/` groups can express child composition; they are optional, and nesting can continue at any meaningful level. Keep each subject's implementation, interface, spec, styles/tokens and related resources together using [purpose suffixes](../../code-structure.md#naming-and-visibility-cues). Do not scatter them into top-level file-kind catalogues or make consumers reach into a sibling's private directory.

A subject can group its visual specifications, local tokens, artwork and motion resources in a nested [design-system collection](./design-system.md#shared-and-local-homes). This remains subject colocation: link the shared product foundation and keep functional/interface/state authority in the owning spec and implementation. Do not move private Page resources into the product-wide system solely because they are design files.

## Map responsibilities to framework directories

Choose directory and entry filenames for the actual framework. Preserve these responsibilities and dependency directions; the following is a reference organization, not a required scaffold. Omit unused groups and keep an established equivalent structure.

| Responsibility | Typical home | Allowed dependency direction |
| --- | --- | --- |
| Bootstrap, routing, providers and global configuration | `src/app/` or the framework entry | Composes screens and public capabilities |
| One task view and its composition | `src/screens/orders/screen.tsx` or `src/pages/orders/page.tsx` | Layouts, feature public entries and presentation components; lower layers do not import it |
| Structure owned by that view | `src/screens/orders/layouts/` | Presentation primitives and layout utilities; receives business content as slots |
| Blocks used only by that view | `src/screens/orders/blocks/` | Its domain's public contracts and lower presentation layers |
| Domain Blocks reused by views | `src/features/orders/blocks/` | Feature internals, layouts and common capabilities |
| Feature-local presentation components | `src/features/orders/components/` | Presentation primitives and generic hooks/utilities; keep domain decisions in Blocks or domain modules |
| Feature state, effects, requests and business rules | `src/features/orders/` with `hooks/`, `api/` or meaningful domain modules as needed | Owned types and explicit public service/domain contracts |
| Public feature contract | `src/features/orders/index.ts` or an established explicit public entry | Exports only Blocks, hooks, operations and types consumers actually need |
| Reusable structure | `src/layouts/` or design-system layouts | Presentation primitives and generic layout utilities; no feature or page imports |
| Domain-independent presentation and behavior | `src/components/`, `src/hooks/`, `src/lib/` or existing shared owners | Lower common layers; no feature or page imports |

Name Blocks for business responsibilities, such as `OrderHeaderBlock`; feature-local `components/` can hold presentation pieces before broader reuse is established. Business-specific components remain with their domain even when several features consume them. Export through the owning domain's public contract instead of moving business rules into generic `hooks/` or `lib/` merely to share them.

Keep page/screen code focused on composition and use-case coordination. Framework loaders or server entries may obtain data through domain APIs; they must not become the home of domain rules. Feature consumers use its public entry, never sibling private files. Keep low-level common modules independent of features and screens. Use existing import-boundary linting where available; introduce additional enforcement only when it addresses a real maintenance need.

### Framework examples

| Context | Entry and layout mapping |
| --- | --- |
| React app with explicit router configuration | Register `screens/orders/screen.tsx` (or the established `pages/` equivalent). Compose `screens/orders/layouts/OrderLayout.tsx`; reuse shared layouts from their public owner. Neither filename is a React requirement. |
| Next.js App Router | Keep `app/orders/page.tsx` (optionally under `src/`) as the route entry and `app/orders/layout.tsx` for a persistent shared route layout when needed. Colocate private structure in `app/orders/_layouts/OrderLayout.tsx`, or use an existing external `layouts/` owner. A named `screen.tsx` may hold the view when a separate boundary is useful, but does not replace the required `page.tsx`. |
| Next.js Pages Router | Keep route files such as `pages/orders/index.tsx`; place supporting screens and layouts outside `pages/` because ordinary files there participate in routing. Keep the same composition and ownership boundaries. |

Next.js `layout.tsx` is a framework route wrapper with framework-defined props and lifecycle; a regular layout component is a spatial contract. The route wrapper can compose that component while retaining framework-required concerns, including root document markup. Ordinary React slots do not require Next.js parallel routes. Do not add `'use client'` to an entire page or layout just to organize files; preserve the actual server/client boundary.

Check the project's router and installed version before adapting conventions. Official references: [Next.js project structure](https://nextjs.org/docs/app/getting-started/project-structure), [Pages Router pages and layouts](https://nextjs.org/docs/pages/building-your-application/routing/pages-and-layouts).

## Shared capabilities

| Capability | Ownership and boundary |
| --- | --- |
| Shared state | Group by domain and lifecycle; expose coherent reads/actions. Lift state only to the scope that coordinates its consumers, following [state ownership](./interaction-and-state.md#state-ownership) |
| API clients and SDKs | Organize by service/capability; keep transport and generated clients behind the applicable [integration contracts](../../scenarios/engineering-design.md#contract-boundaries-and-integration) |
| Common methods | Share a stable capability with a clear contract; keep subject-specific calculations beside the subject rather than collecting them in a global `utils` directory |

Shared scope can itself contain nested capabilities. Neither a common store nor an SDK should become the place where unrelated domain policies accumulate.

## Local design descriptions

Keep consequential local choices with their owner. Shared tokens do not explain a Page's hierarchy or a Block's interaction model; a short entry-point section or existing spec can suffice.

| Owner | Describe when relevant |
| --- | --- |
| Application or Page collection | Navigation, Page relationships, shared layout and cross-page workflows |
| Page or Screen | User task, regions, visible states, actions, responsive composition and return behavior |
| Frame | Slots, placement, resizing, scrolling and adaptation responsibilities |
| Block or Component | Local behavior and visual decisions not evident from its public contract; links to state, motion and asset requirements |

Use [local artifact guidance](../../documentation.md#layered-module-context-and-local-artifacts) for larger context collections. Create only the files the subject needs; keep public contracts canonical in source and link shared rules.

Check two things: can a fresh agent find and reuse the right capability from the map, and can a conforming child implementation change without forcing the parent to absorb its private complexity? Use [component and system review](./review.md#component-and-system-review) for affected boundaries.
