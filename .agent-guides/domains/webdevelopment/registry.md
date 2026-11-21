---
schema: scenario
description: Make design-system capabilities discoverable and distributable through registries, contracts and examples
tags: [web, registry, design-system, agent-context]
---

# Component Discovery and Registries

Use this guide when an agent needs to discover existing UI capabilities, when adopting an external registry, or when distributing a project's design system. For React/shadcn projects that need source-owned customization, prefer the shadcn-compatible registry format before inventing another distribution protocol. Keep [component design](./component-design.md) and [design-system decisions](./design-system.md) with their existing owners.

## Separate the responsibilities

| Responsibility | Authoritative material | Question answered |
| --- | --- | --- |
| Design intent and shared rules | Design-system rationale, tokens and interaction conventions | What should this product express and preserve? |
| Available distribution items | Registry catalogue and item definitions | What can be obtained, and which files/configuration/dependencies does it bring? |
| Current local capabilities | Actual source contracts, exports and generated documentation where available | What does this project currently have, including local modifications? |
| Appropriate usage | Component contract, use cases, composition constraints and examples | When, where and how does this capability fit? |
| Rendered evidence | Representative scenes and accepted [Storybook examples](./design-system.storybook.md), or an existing local gallery | What does it look like and how does it behave in relevant states? |

A registry can carry several of these artifacts, but an item listing alone does not supply all of their meaning. Distinguish upstream availability from the current local inventory: a copied component may have diverged from its upstream item.

## How the shadcn mechanism works

The producer defines a source `registry.json` with registry identity and items, optionally composed through `include`. An item identifies its source files, dependent registry items, npm packages and relevant installation additions. Consumers obtain source code and configuration they can adapt. The [registry format](https://ui.shadcn.com/docs/registry/registry-json) and [item schema](https://ui.shadcn.com/docs/registry/registry-item-json) are authoritative.

There are two delivery choices:

- Build item payloads and serve them through a registry endpoint; consuming projects can map a namespace in `components.json`.
- Use a `github.com` repository with a root `registry.json` directly through the supported GitHub address format. Current official documentation covers public repositories and authenticated private repositories; GitHub Enterprise hosts require another delivery approach. See [GitHub registries](https://ui.shadcn.com/docs/registry/github).

Agents can discover and inspect items through the CLI or an actually configured [shadcn MCP server](https://ui.shadcn.com/docs/mcp). Use the [shadcn Skill](../../skills/shadcn/SKILL.md) and its registry reference for execution details. Recheck version-sensitive behavior against official documentation before relying on it.

## What the native schema expresses

| Native fields or types | Useful responsibility |
| --- | --- |
| `name`, `title`, `description`, `categories` | Identity and searchable/descriptive context |
| `files`, file `type` and `target` | Source content and installation destinations |
| `dependencies`, `devDependencies`, `registryDependencies` | Package and registry-item dependencies |
| `cssVars`, `css` | Theme values and relevant styling additions |
| `registry:base` | A complete design-system base, including its supported configuration |
| `registry:theme`, `registry:style`, `registry:font` | Theme, style and font delivery |
| `registry:ui`, `registry:component`, `registry:block`, `registry:page` | Different installable UI/source shapes |
| `docs`, `meta` | Installation documentation and extensible metadata |

Consult the official [examples](https://ui.shadcn.com/docs/registry/examples) for the exact base/style configuration. These types describe distribution; they do not replace our [Page/Screen/Frame/business Block model](./interface-structure.md#responsibility-model). A file `target` determines where code is written, not where the component belongs in a user's screen.

## Make appropriate usage discoverable

Give an item a concise description using the user's task vocabulary, then link its canonical contract and representative examples. Include the conditions that distinguish plausible alternatives, especially for richer patterns and business Blocks.

| Selection information | What an agent needs |
| --- | --- |
| Role and use cases | Primitive, pattern, Frame, Screen or business Block; the task it supports |
| Exclusions | Similar-looking situations for which its behavior is unsuitable |
| Composition | Expected parent context, slots, data ownership and emitted events |
| Behavior and adaptation | Relevant states, keyboard/focus behavior, responsive and theme assumptions |
| Compatibility | Framework/primitive base, dependencies, token conventions and runtime constraints |
| Evidence and provenance | Public contract, example/story, source revision and applicable license information |

Keep executable API details in source declarations. Link or derive discovery metadata instead of maintaining another props specification. A proposed project convention can use the native `meta` extension, for example:

```json
{
  "name": "record-workspace",
  "type": "registry:block",
  "description": "Browse, filter and select records while retaining list context.",
  "categories": ["records", "workspace"],
  "meta": {
    "agent": {
      "role": "business-block",
      "useWhen": ["Browse and select from a record collection"],
      "avoidWhen": ["Edit one record in a standalone form"],
      "composesIn": ["Page", "Screen"],
      "contract": "<canonical contract reference>",
      "examples": "<canonical rendered examples reference>"
    }
  }
}
```

This is an illustrative metadata excerpt; supply actual file definitions and resolvable references before publishing an installable item. `meta.agent`, its keys and their interpretation are a proposed project convention. Stock registry tools do not acquire selection rules or custom-field search behavior merely because these fields exist. Put the short matching intent in standard discovery fields, and instruct the agent to read the custom metadata and linked contract before choosing.

Define how references resolve and ensure consumers can access them. Do not publish producer-local paths as though they exist in every consuming application. If documentation is distributed with an item, reference its actual installed destination.

## Build and expose a design system

1. Establish the minimum [visual and interaction foundation](./design-system.md#establish-the-minimum-coherent-foundation), reusing an adequate existing base.
2. Implement or adapt representative primitives and a richer task composition. Validate that tokens, behavior and assets work together.
3. Keep contracts and meaningful state examples with these components. Provide a shallow discovery entry into the actual local implementation.
4. Package capabilities when they have a real distribution/reuse need. Use a base/theme for the shared foundation and focused component/pattern items for optional capabilities.
5. Expose the catalogue through the simplest suitable delivery route, then make the approved source and reading procedure discoverable from the consuming project's context.
6. Exercise a real consumer to verify source destinations, imports, dependencies, documentation access and composed behavior before calling the registry ready.

A registry serializes and distributes design decisions; it does not choose a visual language or validate a user experience automatically. A small single-product codebase can begin with local source contracts and examples without publishing a registry.

## Agent consumption loop

Use the [local directory map and public contracts](./interface-structure.md#reuse-before-new-boundaries) to discover current capabilities first. For a real gap, search the project's selected registry sources, inspect the resolved item and its examples, and compare the contract to the required scene.

Before installation, inspect file destinations, dependency closure and styling/configuration changes. Use the Skill's preview/diff procedure, then integrate with the actual aliases, design system and ownership boundaries. Check the rendered consumer using [review guidance](./review.md). Update local discovery information when the adopted contract changes.

Record the chosen registry/source and compatibility constraints once in the product context; reuse those decisions while they remain applicable. Namespace, package or provider names are discovery clues, not evidence that all items share a compatible design language.

For reproducible GitHub consumption, use explicit revisions where needed, including dependent items. A bare dependency name such as `button` addresses the built-in shadcn item; same-repository dependencies need their actual registry addresses. Treat upstream changes as source updates to inspect and merge, not as automatic replacement of intentional local adaptations.

## Use open-source systems deliberately

Not every open-source design system is published as a shadcn registry. Some use packages, source repositories and separate documentation; for example, [Mantine's integration](https://mantine.dev/guides/vite/) uses its packages, styles and provider.

Use compatible registries to acquire designed components or patterns when their actual contracts fit. Use other systems as visual/interaction references or adopt their native integration when that is the better product choice. Do not repackage an entire library merely to make every source appear in one catalogue.

Keep one coherent primary visual/primitive foundation and evaluate additions against it. Preserve provenance and license constraints when adapting source or resources, following [component adoption](./component-design.md#integrate-an-adopted-component) and [assets](./assets.md).

## Choose the smallest suitable setup

| Situation | Preferred approach |
| --- | --- |
| One product, few stable local components | Existing source contracts and a useful example/gallery entry; consume external registries only for gaps |
| Interactive component/Screen design, treatment comparison or user feedback | The design system's [Storybook workbench](./design-system.storybook.md), independently of whether distribution is needed |
| Reusable React/shadcn source intended for product-specific customization | A shadcn-compatible registry plus canonical usage contracts and rendered examples |
| Reusable source already lives on GitHub and needs no custom delivery service | Direct GitHub registry; validate the supported authentication and consumer environment |
| Custom hosting, request authentication or delivery behavior is required | Hosted registry with configured namespaces |
| Components should remain centrally maintained runtime dependencies | A package-based component library with its native documentation; use a registry only for suitable setup/composition artifacts |
| Existing or growing Storybook catalogue | Reuse its documentation/stories; consider its MCP access as an additional discovery and validation channel |

[Storybook's MCP documentation](https://storybook.js.org/docs/ai/mcp/overview) currently marks its AI capabilities as preview. Its documentation toolset relies on a generated component manifest and supported framework configuration; testing capabilities require their corresponding setup. Evaluate that integration against the installed version. Keep ordinary contracts and examples usable without requiring a preview integration.

For our current React/shadcn-oriented methodology, the default composition is **registry for discovery/distribution + canonical component contracts for selection/use + rendered examples for evidence**. Use Storybook for the interactive design workflow described above; a distribution-only task can reuse existing examples. CLI access is sufficient initially; add MCP when it improves the actual agent workflow.
