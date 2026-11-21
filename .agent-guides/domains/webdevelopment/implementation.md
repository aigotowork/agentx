---
schema: scenario
description: Implement and debug web interfaces while preserving component, runtime and TypeScript contracts
tags: [web, implementation, typescript, react]
---

# Frontend Implementation

Use the sections relevant to implementation, debugging and integration. For a bounded change, inspect its existing contracts and callers and proceed; do not restart product design. When responsibilities or behavior are unclear, follow the corresponding route in the [frontend entry](./AGENTS.md).

Before UI implementation, read [product and interface design requirements](../../scenarios/interface-design.md), including for bounded fixes. Reuse them when already read. Non-UI TypeScript work does not trigger this requirement.

Apply [continuous preview](continuous-preview.md): expose the affected page early and keep it accessible while implementing. Inspect its actual source/build/cache path so manual refresh reads the current version without unsolicited updates during use.

## Inspect the affected slice

Read the nearest context, target component/Page, public usage and relevant styles, state and data contracts. Inspect actual callers before changing a public boundary. Use a focused source view; the available [tsxctx Skill](../../skills/tsxctx/SKILL.md) can help inspect JavaScript/TypeScript contracts and composition, with its documented fallback when unavailable.

Check the existing framework, package manager, primitive base, styling conventions and build scripts before introducing tools. Reuse accepted project choices. For an unfamiliar or version-dependent API, verify installed versions and current official documentation rather than copying a remembered snippet.

## Build through the designed boundaries

Implement the [composition](./interface-structure.md) and [component contract](./component-design.md) inward. Keep higher-level code readable as named regions and task coordination. Put cohesive calculations, event handling and effects behind their actual owner, following [code structure](../../code-structure.md).

When replacing a [contract placeholder](./interface-structure.md#compose-before-implementing-internals), implement the child's agreed inputs, events and visible guarantees within its owner. Revise the contract and affected callers if the design changes; do not pull private child logic into the parent to bypass a missing implementation.

Use [semantic classes, variables and variants](./design-system.md#token-and-theme-design) for all authored styling, including local presentation. Encapsulate low-level styling in Layouts and Components; assembly code consumes semantic regions and contracts. Use the established [state/effect owners](./interaction-and-state.md) for behavior and the actual [assets](./assets.md) and [motion contracts](./motion.md) for visual requirements. Do not defer missing resources indefinitely behind generic placeholders.

Resolve the shared system and subject-local [design resources](./design-system.md#source-to-consumer-ownership) before adding styles or assets. Wire canonical token/theme imports and actual runtime asset references explicitly, keep local styling scoped, and refresh derived exports through their owning process. A `uiux/` folder alone does not make its resources part of the rendered scene.

Implement the [modeled transitions and effects](./interaction-and-state.md#model-behavior-before-presentation) under their owner; render the resulting state and current inputs through the agreed view projection. Keep guards and domain decisions out of scattered rendering conditions. This separation is conceptual, not a requirement for separate files or a machine library.

Integrate real capability contracts without pulling transport or persistence details into primitives. Preserve error semantics and runtime validation at the external boundary. Treat hidden/disabled controls as user-interface behavior, not as authorization enforcement.

For an HTTP/RPC backend, first locate its maintained API/data definitions and use [API contract and SDK guidance](../../scenarios/api-contracts-and-sdks.md) for the existing or selected client generator. Keep generated transport code behind the owning frontend capability and adapt it to the component/domain protocol. Frontend-local interactions continue to use typed component/module contracts without introducing RPC.

## Prototypes and realistic fixtures

An interactive prototype should expose the scene layout, named task components, shared primitives and representative view-state inputs. Keep simulated data and actions identifiable. Use fixtures that reveal populated, long-content, pending and failure behavior when relevant.

For component and Screen design, follow the [Storybook workbench](./design-system.storybook.md) for story placement, providers, controlled interaction, fixtures and shareable previews. Import canonical components and resources; keep story-only harnesses and experiments outside production entry points. Use accepted stories as implementation examples while checking the actual consuming flow.

Do not implement production persistence or transport merely to demonstrate a design. Conversely, when delivering production behavior, identify and replace simulated actions rather than leaving a working-looking control disconnected. Use the existing owning record to distinguish accepted design, implemented behavior and remaining integration.

## TypeScript and React contracts

These conventions also apply to non-web `.ts` and `.tsx` work; that work does not need unrelated web sections.

- At the top of every `.ts` or `.tsx` file, add a concise file-level JSDoc comment with `@aiPurpose` describing the file's purpose and design goals; for package entry files, describe the package's purpose and design goals. This supplies explicit file-level context for `tsxctx`.
- Keep a `.tsx` component's return expression focused on structure. Put complex calculations and behaviors in named variables or functions before the return.
- Every exported React component, page or module in a `.tsx` or `.ts` file must have a concise JSDoc usage contract immediately above its export. Include `@aiPurpose` and `@aiUsage`; add `@aiBehavior` or `@aiLayout` when they clarify state ownership, callback semantics or composition boundaries.
- Keep each tag value on one line and avoid repeating obvious types or JSX details. Make the contract sufficient to determine when and how to use the export; link consequential design rationale rather than embedding the whole design in a comment.

Keep public usage and source behavior aligned when changing contracts. Preserve deliberate exports and dependency direction rather than exposing private internals to make one caller compile.

## React component boundaries

Apply [layout-first composition and framework mapping](./interface-structure.md#establish-layout-before-content) when implementing a structured view. Keep the JSX readable as named regions and task components.

- Give components names that express purpose, such as `PriceBreakdown`, rather than appearance such as `RedText`. Prefer a meaningful component boundary to a long anonymous JSX region explained by comments.
- Keep simple, tightly coupled private child components in the same file, defined at module scope. Do not define component types inside a rendering parent: their changing identity can reset child state.
- Explicitly type private as well as public props. Pass parent-specific values through props instead of capturing them in a nested component closure. Keep inputs small and cohesive; use a domain value when it is the actual contract rather than passing a large unrelated object.
- Split a complex or independently reusable component into its own file. Promote a private component when a second consumer, independent testing need or unreadable parent file justifies it; export only what consumers need.
- Extract state/effect coordination into an owning hook when useful. Keep pure business calculations in domain functions or modules; not every calculation needs a hook or separate file. Components focus on rendering and coordination through those contracts.
- Prefer native semantic elements and their implicit roles. Use ARIA for accessible names, descriptions and required interaction semantics; a custom `role` prop or a visual component name does not supply HTML semantics by itself.

## Accessible and responsive implementation

Use semantic elements and established accessible primitives that fit the actual interaction. Preserve names, labels, focus visibility and keyboard behavior while customizing styles. Add ARIA according to the adopted interaction pattern rather than using it to patch an unclear behavior model.

Exercise relevant keyboard, pointer and touch actions. Preserve reading and focus order when layouts rearrange; check content growth, text resizing and meaningful narrow/wide scenes. Provide understandable status and error feedback and avoid relying solely on color, hover or motion.

Apply the product's adopted accessibility requirements. When a control requires a specialized pattern or a formal conformance judgment, consult its authoritative specification and verify the actual implementation. An automated scan or an accessible library does not establish the accessibility of the composed page.

## Development servers

Use a non-default port when developing web apps to avoid port conflicts, subject to explicit project/runtime instructions. Reuse a suitable existing project server when appropriate; record the actual address and avoid interfering with another task's process.

Use the project's normal development command and required configuration. Keep credentials out of source, fixtures and public client configuration. Track processes started for the task and clean up task-owned temporary resources when they are no longer needed.

## Runtime and delivery constraints

Select concerns by the affected feature; this is not a mandatory production checklist for every edit.

| Trigger | Address |
| --- | --- |
| Server rendering, hydration or browser-only capability | Explicit server/client ownership, compatible initial output, browser API access and cleanup of effects |
| Remote data or mutation | Existing API contracts, runtime mapping, session/access handling and the [visible state lifecycle](./interaction-and-state.md) |
| Large media, fonts or new dependencies | Actual loading behavior, suitable delivery size, fallbacks and introduced runtime cost |
| Dense data or expensive interaction | Measure the affected scene before choosing pagination, virtualization, memoization or other optimization |
| User content, uploads or rendered rich text | Trust boundaries, appropriate validation/escaping or sanitization and safe resource handling |
| Public discoverable pages | Route semantics, document metadata, meaningful content and the project's indexing/sharing requirements |
| Offline, installed or reconnecting experiences | Explicit availability, cache/update semantics and recovery behavior |
| Shipping a changed flow | Relevant build/configuration checks, direct-entry/refresh behavior and the project's existing deployment/observability expectations |

Set budgets from the product and runtime constraints rather than inventing universal thresholds. Measure when performance is the concern; reducing one bundle statistic does not alone prove a more responsive experience.

## Verify the intended result

For a bug, explain the failure through the relevant design, contracts and retained evidence, then correct its owning cause. For a feature, make the requested user outcome reachable. Design [scenario/state cases](../../scenarios/feature-scenarios.md#design-cases-before-writing-tests) and write the relevant tests. Ordinary implementation does not run them; apply the [execution limits](../../principles.md#quality-tests-and-images). Use [review evidence](./review.md#choose-evidence-by-risk) for an assigned UI design or user-intent review, including an explanation of any image read.

Distinguish type/build results, automated behavior checks and observed browser behavior. Report what was actually exercised and any environment limitation. Do not claim visual, responsive or interaction acceptance solely because the code compiles.
