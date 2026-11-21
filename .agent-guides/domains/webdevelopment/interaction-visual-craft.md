---
schema: scenario
description: Refine layout-led interfaces through coherent state expression, microinteractions, domain assets and reusable motion
tags: [product-design, ui, design-system, motion, assets]
---

# Interaction and visual craft

Apply when designing, implementing or reviewing product UI,
including prototypes and design systems. For a bounded change, apply only the
affected decisions. This composes the shared [interface requirements](../../scenarios/interface-design.md),
[motion](./motion.md) and [assets](./assets.md)
methods with the decisions below. When UIUX is adopted, resource formats remain
owned by `uiux docs design-system/resources`; use the actual host otherwise.

## Let layout explain

**Avoid unnecessary secondary labels and descriptions. Let the layout do the explaining.**

Treat this as a primary design principle. Use position, alignment, grouping,
typography, spacing, recognizable controls and visible feedback to communicate
purpose. Before adding explanatory copy, identify the ambiguity and improve its
owning structure or control. Remove repeated subtitles, obvious instructions and
status sentences already conveyed by a clear visual state.

Keep meaningful names, exact values, units, unfamiliar concepts, consequential
actions and error/recovery guidance readable where needed. Prefer a compact icon
button with an accessible name and tooltip for familiar secondary actions; retain
visible text when the action would otherwise be ambiguous. Put optional explanation
in focus/touch-accessible tooltips and occasional actions in menus/popovers.
Essential decisions, failures and primary actions must remain discoverable.

## Compose the whole, then refine its expression

Use this order within the agreed scope:

1. Establish project navigation and page relationships, task entry and return.
2. Establish named Layout regions, hierarchy, proportions, scrolling and responsive behavior.
3. Compose named business Blocks with the information and actions needed together.
4. Design their key Components, disclosed states, resources and feedback together.
5. Once the complete composition exists, operate it and refine interaction,
   artwork, motion and shared visual rules in a concentrated pass.

Asset needs, motion intent and state meaning participate from the start; reserve
their space and include required behavior in the initial whole. Refinement tunes
the complete design rather than postponing all feedback or artwork until the end.
Follow `uiux docs design-system/prototype-workflow` for broad authoring batches;
do not alternate each control edit with browser polish. If operation reveals a
structural problem, correct its Layout/Block owner before refining decoration.

## Design microinteractions as stateful feedback

For each important interaction, connect trigger, current state, feedback and
result. Use the existing component contract; a short state/trigger table in the
owning design is sufficient where the behavior needs explanation.

| Interaction | Design expression |
| --- | --- |
| Hover, focus, press | Clear affordance, focus visibility and immediate acknowledgement; no layout jump |
| Select, toggle, like | Persistent selected/pressed appearance; brief local feedback that settles into the final state |
| Open, close, switch | Related origin/destination, coherent entry/exit, dismissal and focus return |
| Drag or direct manipulation | Input-linked movement, target indication, release/cancel and stable result |
| Pending, success, failure | Distinct provided states, readable progress/result and actionable recovery |
| Achievement | A proportionate one-shot reward attached to a real milestone, with final progress retained |

Prefer color, emphasis, icon shape, border, fill and small component variants to
repeated status prose. Keep semantic meaning consistent across pages and themes.
Color needs another perceptible cue; expose accessible names/states and relevant
status announcements. Preserve precise values and state details where the task
requires them. A disappearing animation cannot be the only result indicator.

Input acknowledgement may begin immediately; confirmed success must follow the
actual result. If the existing contract uses optimistic state, preserve its
rollback/recovery behavior and reserve achievement effects for confirmed success.
Repeated input, failure or navigation must not strand a pending visual state.

## Build a domain-related visual family

Choose a few concrete qualities from the product's purpose and audience, then
translate them into palette roles, geometry, illustration motifs, imagery,
texture and motion character. For example, a learning product might pair warm
accents and growing forms with gentle achievement feedback; an inspection tool
might use precise outlines and restrained directional highlights. Record the
chosen relationship in the existing system design instead of using generic mood words.

Treat backgrounds, SVG illustration, photography and animated decoration as
design resources. Compose a coherent family, including relevant empty, pending,
result and achievement states. Keep crops, focal points, contrast and whitespace
compatible with the actual Layout. Preserve the established icon grammar and
functional status colors; brand expression must not remap success or error meaning.
Use only categories that serve the chosen composition.

Follow the shared asset brief and provenance method. Keep editable vector assets
in their native form; use an available image-generation skill for needed bitmap
artwork. Distinguish decoration from informative imagery and supply appropriate
text alternatives. Assets and effects must appear in their real consuming states;
an asset gallery alone does not establish integration.

## Give motion and effects a dedicated system layer

Maintain reusable motion/effect definitions alongside the existing canonical
system resources. Share semantic timing, easing, displacement, scale and intensity
roles, plus effect primitives when actual consumers justify them. Keep page-specific
art and sequences with their subject. Product and workbench scenes consume the
same public entries; do not create a second effects implementation for previews.

Separate these purposes when choosing the treatment:

| Family | Examples and limits |
| --- | --- |
| Control feedback | Button press, toggle transition, like burst; keep the control usable throughout |
| Spatial continuity | Menu/drawer entry, tab indicator, item movement; preserve origin, focus and reading order |
| State feedback | Pending rhythm, success check, failure highlight; reflect supplied state and retain a stable cue |
| Ambient expression | Pointer-follow glow, parallax, animated background; stay behind content, avoid input interception and stop when inactive |
| Reward | Confetti, achievement burst; use meaningful confirmed milestones, bounded duration and controlled repetition |

For each substantive effect specify purpose/trigger, subject/origin, start/end
state, duration/easing/delay, amplitude/particle or repetition bounds, interruption,
cleanup and reduced-motion/static alternative. Reuse the shared motion contract;
keep this information with the owning component/resource, not a separate tracker.
For a like burst, for example: trigger on confirmed activation, emit from the
button, settle to the active icon, suppress overlapping bursts, cancel on unmount,
and retain the same active state without particles under reduced motion.

Tune a small timing hierarchy in the real composition: control acknowledgement
should feel immediate, surface transitions should preserve orientation, and a
reward can outlast a tap without blocking the next action. Store actual adopted
values in tokens. Do not impose one duration or easing on every purpose.
Reserve the strongest motion/color contrast for meaningful moments.

Start with existing primitives and native CSS/SVG for simple feedback. Consider
a coordinated motion library for gesture/presence needs, a timeline tool for
complex sequences, or an authored animation format for interactive/vector artwork
only when that requirement exists. Inspect the chosen runtime/API and delivery
cost before adoption; this guide requires no particular library or new dependency.

Honor reduced-motion preferences. Decorative pointer effects need a stable
touch/keyboard alternative and must not hide controls or convey exclusive meaning.
Keep effects nonblocking, release listeners/timelines/particles on exit, avoid
unbounded concurrent effects and pause ambient work when hidden/inactive. Prefer
transform/opacity for simple motion and inspect actual rendering cost for expensive
blur, filters or canvas. Business state and focus must not wait for decoration.
Sound, if separately justified, follows UIUX's default-off consent rules.

## Refine and verify in context

After the whole exists, review representative real content at the target sizes.
First remove redundant explanatory text and repair hierarchy/disclosure; then
align colors, typography, spacing, icon weights, surfaces and asset treatments;
then tune interaction timing, origins, intensity and transitions as one family.
Batch related corrections at their shared owner.

Operate the primary path and relevant hidden states, including rapid repeat,
interruption, failure, focus/keyboard, touch where applicable and reduced motion.
Use executable scenes for reusable effects with replay/reset and state controls.
Include normal, interrupted and static treatments when those distinguish behavior.
Observe the effect over time: screenshots establish composition or key frames,
not timing, responsiveness or continuity. Use the shared [observable-feedback method](../../scenarios/observable-feedback.md)
when timing or retained state matters.

Finish with canonical component/asset/effect entries, adopted tokens, integrated
consumer states, runnable scenes and bounded observations in the existing
design/task. Identify unobserved behavior and simulated results explicitly.
Stop when feedback is readable, the visual family is coherent and the affected
path works at the promised fidelity; no exhaustive effects catalogue is required.
