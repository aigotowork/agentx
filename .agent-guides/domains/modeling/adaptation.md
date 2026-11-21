---
schema: scenario
description: Preserve semantic obligations when projecting behavior into a finite model and interpret the limits explicitly.
tags: [modeling, finite-state, verification]
---

# Adapt a model without hiding its risks

Start from the original scenario criteria and independent contract. State the question and properties before encoding transitions, so a defective guard cannot silently become the requirement. Keep the semantic model canonical and record the checked projection, assumptions, omissions and revision alongside it.

## Establish real steps before finite encoding

Identify fact and resource owners, guards, effects, outcome knowledge and actual atomic boundaries. A transition spanning processes, a permission grant and a physical effect usually needs separate steps: who can prove no execution, when permission becomes irrevocable, and what survives a crash or lost reply? Use [effect boundaries](../../scenarios/data-state-modeling.md#effect-boundaries-and-unknown-outcomes). Do not assert an atomic step merely because it is convenient for the checker.

Choose finite domains that retain the relevant failure. A count abstraction `{0, 1, 2+}` can expose duplicate dispatch while bounding the state graph. Explain the mapping from actual counts and what information it loses. Removing `2+` to fit a range deletes the counterexample; merging states can also introduce a spurious witness. A found path needs interpretation against the original semantics.

## Map each obligation

| Obligation | Finite representation | Required boundary |
| --- | --- | --- |
| At most one dispatch | Prohibit `count=2+` | Does the abstraction preserve every relevant dispatch? |
| A dispatch can occur | Reach a `count=1` goal | Existence avoids vacuous safety, but does not establish eventual completion |
| Each action increments once | Before/after relationship | Needs transition reasoning or a supported edge property |
| Every request eventually completes | Temporal/path property with fairness assumptions | A reachable goal does not prove this |
| Original request and revision agree | Explicit identity/revision model | Fixed-request assumptions do not prove real multi-request handling |

Use the selected checker contract for exact syntax. In Model v1, scalar strings/booleans, constant assignment and state predicates are narrower than arbitrary expressions or temporal logic. Expand dynamic assignment over legal finite values, intersect with original guards, preserve unchanged fields and atomic boundaries, and cover all legal combinations. Do not add a new guard that repairs the bug during adaptation.

For `A implies B and C`, a state-forbid representation generally needs both `A and not B` and `A and not C`; prohibiting only `A and not B and not C` misses violations. Empty predicates and empty domains need explicit interpretation from the tool contract.

If history variables such as `everDispatched` or `previousRevision` are introduced, label them instrumentation. State initial values, update/reset rules, event coverage, atomicity and request lifetime. A recorded flag cannot prove the producer captured true history. Keep unsupported obligations as manual or uncovered, with actual evidence links; do not include them in a mechanical pass count.

## Check, interpret and return obligations

Add representative success, rejection and unknown goals where useful, then run within explicit state/step budgets. Inspect violations and witnesses, unknowns, cutoffs, unexplored frontier and clipped traces. Complete exploration and satisfaction are separate facts. A budget cutoff preserves results already found but cannot silently prove remaining properties.

A supply/all-of check shows that declared prerequisites have a derivation from declared inputs. An `origin` assertion does not validate identity, revision, namespace, freshness or authority. Write these relationships explicitly and choose a check capable of addressing them; marking everything available only checks your assertions against themselves.

Return each result to its original criterion and implementation boundary. Record whether it is mechanically checked under assumptions, manually justified or still uncovered. A finite model does not establish source conformity, real adapter effects or refinement solely because its input hash matches. Finish with a usable mapping and honest result, not a count of green assertions.
