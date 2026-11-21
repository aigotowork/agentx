---
schema: scenario
description: Review user-visible experience and component-system quality with proportionate evidence
tags: [web, ux, ui, review]
---

# Frontend Review

Use this guide for UX/UI assessment, component-system review or acceptance of changed frontend work. Select the relevant scope and evidence; a review is an activity, not a mandatory separate agent or approval stage. Respect the task's authority and distinguish findings from implemented changes.

First read [product and interface design requirements](../../scenarios/interface-design.md). Review the affected interface against these required layout, density, disclosure, visual and semantic implementation constraints; reuse this context when already read.

## Define the scene before judging it

Identify the user, task, entry point, selected object, visible state, content shape, viewport and relevant theme/input preference. Use representative real or clearly labeled fixture data. Choose affected scenes and shared consumers according to risk, not an exhaustive Cartesian product of every option.

A useful evidence frame is:

```text
Given this user context and visible state:
What can the user understand?
What can they do?
What changes when they act?
Can they continue, recover or return?
```

When the design is uncertain, distinguish a hypothesis from observed difficulty or user evidence. A personal aesthetic preference does not establish a usability defect.

## Experience review

Inspect the interface from the user's perspective, then trace important actions into their resulting states.

| Dimension | Examine | Typical failure signal |
| --- | --- | --- |
| Purpose and orientation | Current object/task, location, status and useful next action | The screen exposes controls without a clear reason or starting point |
| Information hierarchy | Relevant facts, grouping, comparison, density and detail access | Decorative cards or fragmented regions obscure relationships |
| Actions and affordances | Discoverability, outcome labels, availability and consequence | A prominent control is inert, ambiguous or disconnected from the task |
| Flow continuity | Selection, drafts, focus, feedback, navigation and return context | The user loses work or orientation after an otherwise successful action |
| Visual expression | Typography, rhythm, alignment, contrast, focal hierarchy and coherent style | Token-consistent pieces still form a visually flat or inconsistent composition |
| State communication | Pending, empty, error, partial and successful outcomes where applicable | A blank screen, indefinite spinner or generic message leaves no useful next step |
| Content and comprehension | Clear terminology, useful examples, units and concise contextual explanation | Repeated prose explains what the UI should make apparent, or necessary guidance is absent |
| Accessibility and adaptation | Semantics, keyboard/focus, reading order, relevant touch/narrow layouts and content growth | The primary task works only with a mouse or at one viewport size |
| Motion | Meaning, timing, interruption, continuity and reduced-motion behavior | Motion distracts, blocks input or leaves an invalid intermediate presentation |
| Assets and credibility | Relevant imagery, legible crops, family consistency, authentic claims and fallbacks | Placeholder or decorative resources undermine understanding or product trust |

Visual polish and functional correctness are both relevant, but their acceptance criteria differ. Evaluate visual decisions against the required interface constraints and the intended product direction. Useful workspace density, semantic composition and token-based styling are requirements; decorative preferences do not override them.

Prefer a UI that explains itself through composition, examples and direct feedback. Do not equate good design with minimum word count: retain necessary labels, recovery guidance and consequential information. Use [experience design](./experience-design.md) when a finding concerns the task model rather than styling.

## Component and system review

Review a changed component at its public boundary and in representative actual consumers. For a system-wide assessment, include a shared primitive, a richer pattern, a domain Block and composed Page usages where those levels exist.

For interface or interaction review, create or update the relevant [protocol diagrams](./interaction-and-state.md#component-and-page-protocols). Trace the applicable action, state confirmation and outcome/recovery paths against the public contract; retain the conclusion and unresolved cases in the existing review/task record. Keep semantic protocol findings distinct from private implementation and visual presentation findings.

| Concern | Review question |
| --- | --- |
| Responsibility | Are Page, Screen, Frame, Block and Component boundaries meaningful, or merely naming conventions? |
| Contract | Do inputs, callbacks, states, slots and variants express actual supported behavior without leaking internals? |
| State/effect ownership | Are data sources and authoritative writers clear, with predictable external updates, reset, cleanup and effects? |
| Behavior and presentation | Do visible regions, available actions and feedback agree with the [modeled states and transitions](./interaction-and-state.md#model-behavior-before-presentation), including relevant failure and recovery paths? |
| Shared versus local | Are stable meanings reused while domain-specific decisions stay local? Are similar-looking but different tasks incorrectly merged? |
| Design-system expression | Do primitives and business components apply tokens, themes, interaction conventions and intentional local design? |
| Interactive design evidence | Do [Storybook examples](./design-system.storybook.md) use canonical components, exercise meaningful states/actions and expose provisional resources or simulation? Can feedback be traced to the reviewed scene/revision, with accepted examples separated from experiments and checked in a composed consumer? |
| Design-resource ownership | Can an Agent discover the shared foundation and local presentation resources, distinguish authored files from derived outputs, and trace them into real consumers without duplicate tokens, private cross-scope imports or leaked local styles? See [resource ownership](./design-system.md#source-to-consumer-ownership) |
| Variants and duplication | Can current needs be expressed without incompatible forks, arbitrary overrides or impossible flag combinations? |
| Composition and dependencies | Can a parent be designed and changed through child contracts without absorbing private child logic? Are imports aligned with those boundaries? |
| Resource integration | Are required assets and motion part of the actual contract and delivery, with suitable fallbacks? |
| Adoption and maintenance | Are copied dependencies, provenance, deviations and update ownership understood? |
| Discoverability and colocation | Do directory descriptions locate usable public capabilities, with each subject's contract, spec, implementation and resources kept together? Is upstream availability distinguished from the current local contract? |
| Completion | Are required child placeholders replaced for an implemented journey, with any intentionally simulated prototype behavior still explicit? |

Use [architecture review](../../scenarios/architecture-quality-review.md) when the concern extends to broader module architecture. Use [component design](./component-design.md) for a contract or reuse remedy, [interface structure](./interface-structure.md) for boundary changes, and [design system](./design-system.md) for shared expression. Do not introduce wrappers, global components or new frameworks merely to make the tree appear more uniform.

## Choose evidence by risk

Quality starts with the user-intent, structure, component, interaction and state models. Use the table below to identify evidence for a declared claim, not as a mandatory execution checklist. Apply the [test and image limits](../../principles.md#quality-tests-and-images). When reading an image for this UI review, explain what it shows and how its hierarchy, content, actions and visible state support or obstruct the intended task.

| Claim to establish | Suitable evidence | Limit |
| --- | --- | --- |
| Types, imports and build compatibility | Existing typecheck/lint/build commands relevant to the change | Does not establish visual or interaction quality |
| Consequential state or calculation correctness | Focused behavior tests for accepted, failed and relevant rejected/overlapping transitions | Does not show users can understand the outcome |
| Component interaction contract | Rendered component or existing story/test with realistic inputs and keyboard/focus checks | May miss problems introduced by its Page consumer |
| Visual hierarchy and adaptation | Actual screenshots or rendered inspection at representative states, content sizes and viewports | A static capture cannot establish timing or flow |
| Journey and recovery | Browser interaction through relevant entry, action, error/retry and return paths | Scope is the paths and environment actually exercised |
| Motion and media | Observed playback and interruption, including the applicable reduced-motion/static case | A selected key frame does not prove continuity |
| Runtime cost | Measurement of the affected load or interaction under relevant conditions | An isolated metric is not a complete performance verdict |

Use existing fixtures and checks when adequate. Design meaningful scenario/state cases before adding tests; justified scoped checks may stay with the implementer, while shared-resource or broader integration checks follow their named owner. Do not create a new test framework, exhaustive matrix or separate review workflow for a reversible low-impact edit. Select affected scenarios under the [execution limits](../../principles.md#quality-tests-and-images); do not repeat established checks for reassurance.

If browser or target-environment access is unavailable, state the limitation and keep static conclusions distinct from unobserved behavior. Automated accessibility tools and screenshot comparisons support investigation; they do not replace meaningful interaction checks.

## Record findings that lead to action

For a substantive finding, record the scene/trigger, observed behavior, user or contract impact, evidence, likely owner and a proportionate correction. Separate:

- Failures that prevent the task, lose data, violate a required contract or create a material accessibility barrier.
- Significant clarity, consistency, recovery or maintenance problems that affect expected use.
- Optional refinements or hypotheses that need further evidence.

Prioritize user and contract impact. Do not block completion on discretionary polish or speculative abstraction improvements. When several symptoms share a cause, fix the owning model, component or system rule and check its affected consumers.

Report completion against the requested outcome, including required resources and actual integration. Record unresolved limitations in the existing owning task/design and link evidence. Component-level completion must not conceal an incomplete user journey.
