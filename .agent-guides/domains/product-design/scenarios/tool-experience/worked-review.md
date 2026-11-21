---
schema: document
description: Constructed review demonstrating wrong use-site scope, version drift and incomplete return despite abundant controls.
---

# Worked review: batch editing and comparison

This is a fictional review illustrating the [method](../tool-experience-review.md). It contains no actual product observation, measured improvement or participant evidence. Replace its assumed promises and native capabilities with original consumer sources before conducting a real review.

Assume the chosen promise: select two use sites of reusable objects, edit a precise property at pinned versions, compare a candidate, and apply only to the selected uses. Other uses and reusable definitions remain unchanged. The user may visit related resource management and resume the same editing task.

## Task plan and responsibilities

Plan: select → verify scope → adjust/find resources as needed → compare → adopt or discard → verify/continue. Invalid input stays editable; unsuitable resources return to comparison; a changed baseline requires conflict handling. Retry only items whose native recovery permits it.

| Subgoal | Facts and UI support | Responsibility to verify |
| --- | --- | --- |
| Select two uses | Stable use IDs, selection count, active item distinct from set, permission/eligibility | Collection view owns selection; stale active item is not the implicit batch target |
| Adjust property | Original/common/mixed values, units, real step/bounds, reset and preview | Domain stores required precision; input round trip changes the intended value |
| Compare candidate | Candidate identity and pinned baseline, visible differences and adopted choice | Displayed and submitted identities agree; mutable latest is not the baseline |
| Find resources and return | Original selection/version/draft, comparison and useful location | UI task owns editing context; resource catalog owns resource identities |
| Apply and verify | Selected uses, expected versions, changes and per-item results | Native operation rechecks scope/version and distinguishes completion/conflict/unknown |

An internal capability need not become a toolbar button. A read-only evidence view can satisfy its task without editing, duplication or deletion controls. Required batch precision here follows the chosen task, not a global requirement for every tool.

## Finding: many controls, wrong use-site effect

Suppose selection shows uses `U2` and `U5`, but Apply reads the previously active reusable definition `A1`. The UI can contain many meaningful controls and still violate the promise. The cause is the binding between selected scope and native effect.

The cognitive walkthrough's association question exposes the ambiguity: the action is discoverable, yet its object/scope cannot be inferred correctly from the visible selected set. The result message also fails if it names only “success.”

Proposed correction: bind explicit use IDs and versions through the application boundary, show scope beside Apply, and report matching per-item outcomes. Preserve direct access to frequent actions. If the native operation only supports changing a reusable definition, this is also a contract/product decision; relabeling alone cannot supply use-site behavior.

Decisive recheck: select two different uses, retain an unrelated active definition, apply, and inspect actual input/result plus unchanged other uses/definition. Include empty selection and mixed eligibility if they affect this promise. Counts, source component ratios and screenshots cannot establish actual scope.

## Finding: displayed and executed versions diverge

Suppose the comparison pins `R11`, while Apply reads global `currentRevision`. After a background change to `R12`, the visible preview still appears to represent the submitted basis. The wrong binding can make a visually polished review misleading.

Proposed correction: submit the selected object/version and validate its applicability. If the service supports only latest-version execution, explain the changed basis and require the product's actual review/reselection flow. Do not pretend arbitrary pinned-version execution exists.

Decisive recheck: select `R11` → change current to `R12` → submit → inspect native input/result and conflict feedback. Reading a suspicious variable in source is an investigation lead; execution or a controlled state check is needed to establish behavior. A static image cannot show which version a service read.

## Finding: draft survives, working context does not

Suppose navigation to resource management preserves the numeric draft but returns to the first object and first candidate, losing selected uses, pinned baseline and comparison position. Preserve the retained draft as a strength, while rejecting “complete task restored” under the assumed promise.

The return contract includes work identity, selected set, baseline/candidate/adopted choice, draft, location and useful focus. On return revalidate objects, versions and permission. If an object disappeared, retain recoverable work with its original scope and explain reselection; do not silently apply it to a new default.

Decisive recheck in one context: enter an unsaved value → choose nondefault baseline/candidate and location → open resources → return → assert each relevant context item. Repeat the meaningful invalid-object/version branch. Do not compare two independently reset snapshots as preservation evidence. Record repeated search/selection/input without claiming a universal switching penalty.

## Precision, partial result and recovery

If the UI displays hundredths but the stored model rounds to whole units, inspect the actual task precision and round trip. A smaller `step` attribute does not fix storage. Mixed values need an explicit representation; do not display zero as a common value and accidentally overwrite both selections.

If one item applies, one conflicts and a third response is lost, retain three separate outcomes. Reconcile the unknown item through its original operation identity. Retrying the entire set can repeat completed effects; closing the waiting panel does not cancel them. Undo must reflect native support rather than a universal UI promise.

## Visual and navigation judgment

With the same representative data and viewport, inspect whether scope, candidate, baseline, precise controls and result can support one judgment together. Local disclosure can retain the main work surface while exposing detail. Putting every control on screen can instead create search interference.

A separate resource-management page is reasonable for an independent cross-project organization task, with a return contract. Narrowing the viewport alone does not justify scattering one editing task across pages. Useful observations include “scope sits beside submission” and “comparison labels remain visible.” Focus return and actual continuity require operation, not screenshots alone.

## Boundaries and dispositions

| Nearby case | Proportionate method/result |
| --- | --- |
| Full batch editing and return | Task table, identity/scope checks, return contract and actual boundary rechecks |
| Independent organization permissions | Separate task/page can be appropriate; examine authority and return |
| Small hover-color correction | Reuse scenario; inspect token, contrast and affected focus/hover state |
| Read-only research view | Judge finding/comparing evidence; missing CRUD is not a defect |
| Static design only | Report visible facts and expert predictions; execution/persistence/focus remain unverified |
| External effect without native undo | Explain actual result and supported prevention/compensation; do not invent full undo or safe retry |

The walkthrough shows that the method can express these distinctions. It does not establish product defects, automated execution, user success or better cognition/aesthetics. A real review closes with original-source evidence, owned dispositions and appropriate scoped rechecks.
