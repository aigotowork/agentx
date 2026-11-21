---
schema: scenario
description: Use Storybook as the design-system workbench for component and Screen exploration, interactive feedback and maintained examples
tags: [web, design-system, storybook, components]
---

# Interactive Design with Storybook

Use this workflow when designing or materially refining primitives, patterns, Blocks or Screens, comparing visual treatments, or collecting feedback on interactive UI. Storybook is the default workbench for this design-system activity. Reuse an existing installation and an established direction; a bounded correction needs only the affected stories, not a new catalogue or a complete design cycle.

## Purpose and ownership

The [design system](./design-system.md) owns the shared design language and this workbench capability. Stories make that language observable and usable: Agents can inspect real states, designers can compare treatments, and users can operate the result before a full product integration exists. Product quality still depends on deliberate hierarchy, behavior, typography, color, motion and resources; installing the tool does not choose those qualities.

Keep component implementations, public contracts and local design decisions with their [subject owners](./interface-structure.md#let-source-structure-express-the-model). Import those implementations into stories. The workbench can assemble a Screen or Page with providers and fixtures without moving its business model into shared primitives or maintaining a second implementation for the demo. Registry discovery/distribution remains a [separate capability](./registry.md).

## Start from design intent and behavior

Identify the user task, relevant [state/action model](./interaction-and-state.md#model-behavior-before-presentation), shared foundation and the design question still unresolved. Translate specified principles, style instructions, reference images or existing products into concrete choices: hierarchy, density, typography, palette/contrast, surfaces, asset character and motion intent. Record which qualities transfer to this scene and which do not; link references in the existing visual brief rather than reproducing an entire reference page.

Use realistic content and the [actual resource brief](./assets.md#resource-brief). Decide [motion purpose and interruption](./motion.md#motion-contract) with the interaction, not after drawing a polished default state. If resources or behavior are provisional, label the gap and what the experiment can establish. Prototype completion, design acceptance and production integration are distinct outcomes.

## Model stories by meaning

| Dimension | Expression | Constraint |
| --- | --- | --- |
| Behavior state | Named story with representative inputs or a reproducible interaction | Follow the component contract; do not expose private state merely for a screenshot |
| Supported visual variant | Existing semantic variant expressed through args | A real consumer need, not every discarded design idea turned into a production prop |
| Environment | Theme, viewport, locale or provider setup | Use shared configuration and representative combinations, not every possible permutation |
| Design candidate | Clearly labeled experimental story and, if needed, a local experimental implementation | State the hypothesis and compare against the current treatment with equivalent content/context |
| Revision and maturity | Source/build revision plus experimental or accepted usage guidance | A story name such as `Version2` is not version history or proof of acceptance |

Use [Component Story Format](https://storybook.js.org/docs/writing-stories) and meaningful story names for the installed framework. [Args](https://storybook.js.org/docs/writing-stories/args) and Controls expose supported inputs; a snapshot of inputs alone does not describe a transition. Cover the important default, focus/selection, pending, empty, error/recovery and content-boundary cases where relevant. Presentational components need only useful input-to-view examples.

## Place the workbench in the design system

For a new setup, use `uiux/storybook/` for the shared workbench, its configuration and system-wide showcases. Keep an existing root `.storybook/`, workspace package or equivalent when suitable; link it from the design-system entry instead of creating a parallel installation. This optional example expresses ownership, not a mandatory scaffold:

```text
<product>/
  uiux/
    storybook/
      README.md                        # run/share entry and catalogue navigation
      .storybook/
        main.ts                        # framework, story discovery and build setup
        preview.tsx                    # shared styles, providers and decorators
      foundations/
        Colors.stories.tsx             # imports canonical token/theme sources
  src/pages/Checkout/
    CheckoutScreen.tsx
    CheckoutScreen.stories.tsx          # imports the actual Screen
    uiux/
      CheckoutScreen.visual.md
      experiments/                     # scoped alternatives only when needed
```

Primitive and Block stories likewise stay beside their implementations, or in an established subject-local design area. Configure discovery explicitly; filesystem placement does not automatically add a story. A development workbench may discover Page-local stories; shared runtime components must not import Page-private resources or story fixtures. Keep story-only dependencies out of production entry points.

Point development and build scripts to the same configuration directory using the installed CLI's [`--config-dir`](https://storybook.js.org/docs/api/cli-options); [story paths](https://storybook.js.org/docs/configure#configure-story-loading) are resolved relative to that configuration. Use the project's package scripts and [server conventions](./implementation.md#development-servers). Import canonical tokens, fonts, styles and assets using the existing source-to-consumer chain; the preview must not acquire its own theme copy. For an existing framework-specific setup, retain its compatible builder, aliases and asset handling.

## Make the design operable

Use [decorators](https://storybook.js.org/docs/writing-stories/decorators) for the necessary layout, theme or context, and scoped fixtures for domain data, routing and services. For [Screen/Page composition](https://storybook.js.org/docs/writing-stories/build-pages-with-storybook), reproduce the environment needed by the task while keeping component responsibilities intact.

A controlled component needs a stateful harness or args update so its events produce visible changes. An action log alone cannot demonstrate editing, selection or confirmation. Simulate unavailable services at their established boundary; use the project's mocking approach, such as [MSW network handlers](https://storybook.js.org/docs/writing-stories/mocking-data-and-modules/mocking-network-requests) for HTTP, without replacing the interaction being evaluated. Make fixture scenarios repeatable and resettable, and label simulated navigation, saves or failure results.

Use a [`play` function](https://storybook.js.org/docs/writing-tests/interaction-testing) when replaying a meaningful sequence or asserting an outcome makes the state reproducible. Keep manual interaction available and distinguish a declared sequence from one actually executed. Check keyboard/focus, content growth, relevant themes/viewports and reduced-motion behavior according to the affected design; see [review evidence](./review.md#choose-evidence-by-risk).

## Iterate with visible feedback

Treat this as a short loop; reuse established results rather than making each row a separate delivery phase.

| Point | Useful working result |
| --- | --- |
| Frame the choice | Task, style/reference interpretation, state coverage and the particular quality or uncertainty to improve |
| Build and compare | Reachable stories using the actual component, realistic content/resources, and a few candidates only where a choice remains |
| Let the user operate it | Exact preview/story links, relevant environment and a short action to try; clearly identified simulation and unresolved parts |
| Interpret feedback | The observed scene/behavior, user's comment, affected design or contract, decision and next change |
| Consolidate | Chosen treatment in canonical implementation/tokens/resources, maintained representative stories, and a check in its real consumer |

Keep feedback tied to a story and source/build revision, including theme, viewport or scenario when material. Distinguish aesthetic preference, comprehension, interaction failure and contract gaps. Ask for user choice where requested or genuinely unresolved; do not invent approval or add an approval gate to every edit. Continue authorized work while feedback is pending and record which decision remains open.

Preserve accepted design reasoning with the owning visual specification and link story examples. Keep live feedback, examined cases, results and next action in the existing [working record](../../scenarios/project-context.md#make-method-execution-explicit), without a new template or a second task tracker. Remove superseded experimental entries from the maintained catalogue when no longer useful; version control retains history. Keep intentionally unresolved experiments visibly separate from accepted examples.

## Share and establish completion

Offer a working development URL for users who can reach it, or build and share through the project's authorized preview hosting. A [static Storybook build](https://storybook.js.org/docs/sharing/publish-storybook) is an interactive browser application; it still needs its assets, providers and mocked dependencies. A localhost address is not a remote review link. Verify the served build and an important story before claiming a preview is available.

Use the existing conversation/task to capture feedback; Storybook does not require a hosted commenting or visual-regression service. Add such integrations only for a demonstrated collaboration need. A successful build establishes neither visual acceptance nor working production services. For design delivery, report the reviewable scenarios, decisions and remaining simulations; for production delivery, also verify the selected treatment in its real application flow and resolve required placeholders.

Retain stable stories as examples for reuse and checks for future change. Inspect a composed consumer when shared foundations change; isolated primitive stories cannot establish the quality of the Screen they compose. Keep ordinary stories, source contracts and usage links sufficient for Agent discovery without requiring an additional AI integration.
