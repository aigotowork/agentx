---
schema: document
description: Derive executable decisions from principles and express them with minimal ambiguity
---

# Write usable guidance

Use when expressing a designed method. [Scenario design](scenarios/scenario-design.md) owns its rationale and fit; [general expression guidance](../../documentation.md#structured-and-model-based-expression) owns the shared writing conventions.

## Principles, deductions and action

Derive action from `principle + situation and constraints -> decision -> action and expected result`. Check the assumptions connecting these parts. A broad principle rarely determines one action in every context.

- State the governing idea briefly when it helps explain a choice.
- Express consequential choices as conditions and actions. Prefer domain terms and concrete verbs.
- Give ordered steps where one result is genuinely needed before the next action.
- State completion or stopping conditions where otherwise work could expand indefinitely.
- Keep necessary exceptions and failure behavior close to the rule they constrain.
- Retain substantial rationale in the owning design. Include only the explanation needed to interpret or apply the guide.

For example, “Use progressive disclosure” leaves the reading decision unspecified. A more usable form is:

> Keep entry points sufficient to choose the next action. Route each situation to the method needed before that work starts. Apply all matching routes unless they are explicitly alternatives. Reuse context already read; load newly relevant guidance when the task changes.

This example defines behavior without requiring every topic to have a separate file. Additional routing semantics belong in [composition](scenarios/composition.md).

## Choose a form by the relationship

| Relationship | Form | Preserve |
| --- | --- | --- |
| Principles that apply together | Short parallel statements | Scope and the constraint being protected |
| Independent conditions and actions | Conditional list or routing table | All applicable cases, including combinations |
| Ordered alternatives; first adequate option wins | Decision ladder | Priority, adequacy and stopping condition |
| One choice changes the next question | Decision tree | Branch meaning, unknown inputs and fallback |
| Several factors determine an action | Decision table | Relevant combinations and defaults |
| Real sequential dependencies | Short steps or sequence view | Required intermediate results and failure handling |
| Legal behavior depends on state | Transition table or state model | State, trigger, guard and result |
| A boundary is easy to misinterpret | Brief example and counterexample | The rule illustrated and why the cases differ |

Use prose when it is clearer. Do not convert two clear sentences into a diagram or force independent obligations into a first-match ladder. A model is useful when its meaning is explicit, not merely because it looks structured.

## Compress without losing decisions

- Use direct English, active voice, consistent terms and one main instruction per sentence or list item. Keep explicit relationships between instructions.
- Remove filler, repeated rules and background that does not help the current reader. Link the owning source instead of maintaining another version.
- Preserve conditions, negation, order, authority, required results and uncertainty. Expand wording when compression creates ambiguity.
- Prefer familiar domain terminology. Define necessary symbols; avoid invented abbreviations and unexplained shorthand.
- Let names and placement express ownership. Explain non-obvious constraints rather than narrating a directory tree.
- Do not optimize a guide for line count, number of files or telegraphic style. The useful economy is less reading and guessing while retaining correct action.

“Understand the context” needs a scope when the relevant context is not obvious. “Inspect the module entry and public contract before changing its boundary” gives a recognizable trigger and bounded action. An example must not silently promise more, remove requirements or broaden authority beyond the rule.
