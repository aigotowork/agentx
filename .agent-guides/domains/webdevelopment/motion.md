---
schema: scenario
description: Design purposeful motion, transitions and expressive effects with explicit behavior and fallbacks
tags: [web, motion, interaction]
---

# Motion Design

Use this guide when adding or changing transitions, gesture feedback, animated components or media. Motion is part of the component and experience contract. Define its purpose before selecting a library or copying an effect.

## Choose the purpose

| Purpose | Useful question |
| --- | --- |
| Feedback | Does the user perceive that an action was accepted, remains pending or has completed? |
| Continuity | Does movement clarify where an object went or how two views relate? |
| Hierarchy and attention | Does the transition reveal an important change without competing with the primary task? |
| Direct manipulation | Does the visual response remain connected to input, cancellation and the resulting state? |
| Product expression | Does the effect support the agreed visual identity without obstructing reading or action? |

An expressive marketing scene and a dense operations tool may need different motion. Do not add animation merely because a component package offers it, or assume that all branding effects are invalid. A deliberate static treatment is a valid outcome.

## Motion contract

For an effect that matters, define the relevant parts near the consuming design/component and inherit shared values from the [design system](./design-system.md).

| Part | Specify |
| --- | --- |
| Meaning and trigger | User/system event and the state change or expressive goal |
| Subject and geometry | Elements, start/end relationship, clipping, layout and overlay interaction |
| Timing | Duration, easing, delay, sequencing and repetition where needed |
| Interruption | Rapid input, reverse action, cancellation, navigation and repeated triggering |
| Interaction | Focus, scrolling, pointer/keyboard availability and whether content stays readable |
| Adaptation | Reduced-motion alternative, relevant device/input differences and static fallback |
| Runtime | Ownership, cleanup and relevant rendering/loading constraints |

Keep the business state transition separate from decorative timing. An interrupted entrance effect must not leave essential content unreachable or an invisible overlay blocking input.

## Select the smallest implementation

Use existing motion primitives and shared values first. Native styling can express straightforward transitions; a coordinated animation mechanism may be useful for gestures, sequencing or complex presence behavior. Prebuilt motion components can fill a specific gap after [component adaptation](./component-design.md#reuse-and-adaptation). Animated artwork follows the [asset pipeline](./assets.md) as well.

Verify the chosen tool's actual API and compatibility when introducing it. Do not prescribe a universal animation stack or bring several libraries into the same scene for overlapping effects.

Keep common timing/easing and reusable effect definitions in the shared system; place local motion specifications, animation sources and exports in the consuming subject's [design-resource collection](./design-system.md#shared-and-local-homes) when useful. Keep runtime triggers, state authority and cleanup with the component implementation, linking the resource definition. Avoid effects that require the whole Page to know a child's private DOM or library timeline.

## Preserve meaning and usability

Honor reduced-motion preferences with a treatment that preserves information and outcomes. Define whether nonessential motion is removed, shortened or replaced by an immediate state change. Essential meaning must remain available without animation.

Keep focus behavior and reading order coherent during entry, exit and reordering. Do not delay a usable action until an ornamental sequence finishes. Give prolonged or repeating media appropriate controls where required by the product's accessibility contract.

Inspect the actual effect with rapid repeated actions, interruption, relevant viewport sizes and reduced motion. Check for lost focus, unintended layout movement, excessive rendering work and unresolved loading. A screenshot can show a key frame; it cannot validate timing or continuity.

Record unresolved visual or runtime limitations with the affected component. Use [review](./review.md) to distinguish a functional defect, an accessibility barrier and a discretionary visual refinement.
