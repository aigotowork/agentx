---
schema: document
description: Specify reproducible teaching experiments whose controls expose causal rules, counterexamples and limits
---

# Interactive experiments

Use when changing input, event order, failure point or design choice helps the reader understand the mechanism. A simulator, game or explorer may be the primary artifact. Select its form through [expression design](../planning/expression-design.md) and use [executable explanations](executable-explanations.md) for a dedicated task experience.

## Establish the value of interaction

| Need | Static candidate | Useful added interaction |
| --- | --- | --- |
| Compare fixed alternatives | Table and aligned figures | Vary the conditions selecting an alternative |
| Follow one operation | Sequence | Reorder, pause or fail a consequential step |
| Understand a threshold | Worked values or plot | Explore the boundary and its consequences |
| Learn migration order | Staged views | Investigate intermediate compatibility and recovery |
| Learn a strategy | Worked challenge | Predict, try, inspect consequences and adjust |
| Experience how rules affect behavior | Trace or recorded demonstration | Actually use/play the mechanism, retry and relate feedback to rules |

Specify the gain before implementing controls. Keep the article-specific model cohesive; do not build a general visualization platform before there are real shared consumers. Reuse existing primitives and suitable installed libraries before adding dependencies.

## Specify the teaching contract

Keep these decisions in the existing brief or Task. Omit only parts irrelevant to the actual model.

| Part | Make explicit |
| --- | --- |
| Learning question | What can the reader explain, predict or do after operating it? |
| Scope | Teaching simulation, actual algorithm or real integration; omitted behavior |
| State/inputs | Entities, identities, initial values, allowed ranges, event schedule and seed if relevant |
| Rules | Each event's guard, state change, side effect and invariant |
| Controls | Which rule variable or ordering each reader action changes |
| Observations | Durable state, saved observations, trace, refusal and invariant result |
| Contrast | Baseline and alternative under the same relevant conditions |
| Decisive cases | Normal behavior, distinguishing counterexample, boundary and relevant failure/replay |
| Recovery | Reset, repeat and useful rewind/restore behavior |
| Unavailable-medium route | Representative initial state, events, outcome and interpretation |
| Acceptance/budget | Required checks, target devices, payload/startup allowance, visible data limits and cleanup |

Controls must answer the learning question. Explain units and input ranges. Keep observation separate from interpretation: a predetermined “safe” label cannot substitute for exposed state and trace.

Begin with a meaningful default or preset near the central question. Reveal additional complexity as needed; a blank configuration panel can make readers learn the interface before they can inspect the mechanism.

## Keep rules and views consistent

Represent state and transitions explicitly, deterministically where possible. Derive trace, diagram and invariant result from the same model. Avoid a second business rule in the visual layer. Isolate and label deliberately flawed baselines.

Use virtual time and explicit seeds when time or randomness matters. Define what one step means; an animation frame is not a transaction. Supply a reproducible distinguishing preset. Keep workloads and failure schedules equivalent across designs wherever their legal events permit. Explain disabled actions instead of silently reordering a reader's requested schedule.

Reuse actual pure selectors when safe and faithful to the lesson. Real browser layout can be preferable when explaining CSS. Do not import credentials, server-only services or production mutation paths to make a widget feel authentic. If a teaching simplification differs from production, state the difference beside the component and in its model documentation.

The component executes observable rules. Prose, labels, captions or feedback explain their meaning. Neither role should hide the assumptions needed to assess the conclusion.

## Worked contrast: the last slot

Teaching values: capacity `C = 1`, allocated count `U = 0`, two independent requests `A` and `B`; invariant `0 <= U <= C`. The separate-check design records eligibility when reading `U`, then later trusts that saved observation at insert. Schedule `A read, B read, A insert, B insert` ends at `U = 2`.

The alternative performs `check U < C and increment U` as one indivisible teaching event; the second attempt refuses and `U = 1`. Display both request lanes, saved observations, committed count and ordered trace. Let readers select design, step, replay the counterexample and reset. A challenge can ask them to exceed capacity, then replay equivalent attempts against the alternative. Reward or explain invariant outcomes, not clicking speed or uncontrolled timing.

This supports a claim about the model's atomic boundary. Database isolation, SQL correctness, retry identity, pending reservations and external effects need their own evidence. The [full worked plan](../examples/contended-resource.md) retains those limits and an accessible trace.

## Accept the model and the experience

| Area | Required evidence |
| --- | --- |
| Semantics | The decisive counterexample distinguishes mechanisms; boundary and repeated events obey stated rules |
| State consistency | Reset/replay restores the same state and trace; rapid input introduces no hidden transitions |
| Understanding | The explanation supports prediction, inspection of cause and identification of limits; actual learning claims need reader evidence |
| Accessibility | Named controls, keyboard operation, visible focus, ordered text output and non-color cues |
| Motion | Step/pause where needed, reduced-motion behavior, essential facts available outside animation |
| Device/theme | Controls and observations remain useful at selected narrow widths and supported themes |
| Failure/no JavaScript | A route to the central relation and conclusion, with any lost exploration stated |
| Runtime | Relevant production source/build, hydration and actual browser path checks |
| Projection | Search/text/Markdown claims match the implemented adapters and policy |

Select the implementation owner's accessibility, UI and testing methods before coding. Test the semantic contrast and meaningful reader path; heading-presence tests do not validate the model. Establish budgets from the artifact and measured host baseline rather than inventing publication-wide milliseconds. Load expensive optional scenes only where needed, stop animation/observers when unused and handle rendering failure.

Default to local synthetic data without external effects. A real integration demonstration requires the actual task's authority, isolated resources, procedure and cleanup. An article does not authorize email, payment or production-tenant changes.
