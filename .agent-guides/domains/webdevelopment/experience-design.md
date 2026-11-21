---
schema: scenario
description: Design user tasks, information architecture and coherent visible-state experiences
tags: [web, ux, product-design]
---

# Experience Design

Use this guide when deciding what a product, page or interaction should enable and communicate. UX owns the user's path and understanding; [visual design](./design-system.md), [composition](./interface-structure.md) and [interaction state](./interaction-and-state.md) realize that experience together.

First read [product and interface design requirements](../../scenarios/interface-design.md). They govern layout first, information/action density, disclosure and visual expression for this work; reuse them when already read.

## Start from the user's task

Identify the actor, situation, selected object, intent, decision and successful outcome. Distinguish primary tasks from secondary actions and occasional configuration. Use existing research, product evidence and user instructions; label assumptions instead of inventing user research.

For a meaningful flow, connect entry → orientation → action → feedback → next step or exit. Include return visits, interruption and recovery where they affect the outcome. Use a short journey, action/state table or wireframe when it exposes a real uncertainty; a routine change can reuse the current model.

A useful scene model is:

```text
Scene = user intent + visible context + information hierarchy
      + available actions + feedback + next/return path
```

This is a design lens, not a mandatory data schema. “The API succeeded” does not specify whether the user knows what changed or what to do next.

## Organize information around work

Group information by the objects and decisions users care about. Choose list/detail, comparison, timeline, editor/preview, workspace or stepwise flow according to the task; avoid flattening every capability into a long stack of cards.

- Make the current object, location and primary action apparent. Keep supporting detail near the decision it informs.
- Preserve selection, filters, drafts and return context when users move between related views.
- Use progressive disclosure for secondary complexity while keeping necessary choices and status discoverable.
- Require the [workspace information/action efficiency](../../scenarios/interface-design.md#design-layout-first-require-information-efficiency): relevant facts and primary controls stay together, with secondary/tertiary actions reachable locally. Compare actual viewport capacity and action access; smaller typography or reduced padding alone does not establish useful density.
- Give navigation, search, filtering and empty states distinct jobs; a missing result, first-use state and unavailable service need different explanations and next actions.

Determine the user's information needs, then establish the layout skeleton before implementing functional components. A visually balanced layout can still hide the information needed to act.

## Design from visible states

For each consequential state, consider what the user sees, understands and can do, and how the next action changes that experience.

| Situation | Design question |
| --- | --- |
| First arrival or no data | Is the purpose clear, and is there a useful first action without a wall of instructions? |
| Populated or dense content | Can users scan, compare, locate detail and retain orientation? |
| Loading, refreshing or submitting | Is progress intelligible? What remains usable? Is the current context preserved? |
| Invalid input, failure or lost access | Does the explanation identify the affected task and a possible recovery without discarding useful work? |
| Partial success or stale data | Can users distinguish completed, pending and uncertain outcomes? |
| Completion, cancellation or return | Is the outcome visible, and can users continue or return to a meaningful context? |

Select applicable states; do not create impossible combinations solely to fill a matrix. Translate required behavior into [state ownership and transitions](./interaction-and-state.md).

## Content is part of the interface

Let hierarchy, grouping, affordances, representative examples and direct feedback explain the product. Apply the required [compact action and explanation preference](../../scenarios/interface-design.md#trust-users-make-the-interface-explain-itself). Remove paragraphs explaining obvious controls and repeated card descriptions; preserve concise names, constraints, consequences and error recovery at the point of use.

Choose terminology from the user's domain and use it consistently across navigation, fields, actions and feedback. Action labels should communicate outcomes. Give data its needed units, time context and provenance. Avoid fake counts, endorsements or success claims added only to make a screen look complete.

Design with realistic lengths, populated examples, empty values and difficult cases such as long names or translated copy. Use fixtures honestly; production data and authoritative content need their actual sources.

## Adapt the task, not only the width

Keep the core task's context and controls together at the target width. Adapt the layout and use local popovers, collapsible regions and drawers for supporting information and secondary actions. Do not turn a desktop workspace into sequential mobile Screens merely because the viewport narrows; reserve a separate Screen for a substantially different scenario or working context and preserve selection, drafts and the return path. A CSS breakpoint alone does not create a new semantic Screen.

Account for keyboard, touch and pointer use, visible focus, reading and focus order, text resizing and relevant localization constraints. Do not rely on hover, color or motion alone to communicate required meaning. Follow adopted accessibility requirements and established patterns; use [implementation](./implementation.md#accessible-and-responsive-implementation) for realization.

## Connect design to evidence

Use a navigable skeleton when the unresolved question is sequence or discoverability; use a rendered composition when the question is density or hierarchy; use a representative high-fidelity scene when it is visual direction. Choose the smallest artifact that can answer the question.

Keep assumptions and consequential choices with the local owner. A static image can support visual judgment but cannot establish keyboard behavior, recovery or flow continuity. Apply [experience review](./review.md#experience-review) to the real states and actions, and distinguish observed problems from hypotheses needing user evidence.
