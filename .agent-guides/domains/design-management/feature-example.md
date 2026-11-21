---
schema: document
description: A fictional report feature demonstrates owned design sources, a bounded change and proportional maintenance.
---

# Worked example: maintain a report feature

This is a fictional design, with no implemented service, user study or passing runtime checks. Each section stands for an existing owner's source; keeping them in one document makes the example portable. A real product can link its own documents and declarations without copying them into this layout. Use [the workflow](workflow.md#maintain-a-features-current-design) and [relationship meanings](views.md#relationships-between-design-expressions) for the method.

## Report export

An analyst needs to generate a report while doing other work, then retrieve the original result. Reducing waiting is a value hypothesis awaiting actual use evidence. The selected initial scope is an analyst's own reports through Web; sharing, scheduling and batch export remain outside it.

The continuing feature connects these sources:

| Design source | What it owns |
| --- | --- |
| [Export and return scenario](#export-and-return-scenario) | The meaningful path and its observable criteria |
| [Run rule](#run-rule) | Run identity and lifetime; the domain owner retains the precise operations |
| [Result view](#result-view) | Navigation, presentation choices and fixture-based prototype limits |
| [Support and next questions](#support-and-next-questions) | What is defined, what remains unresolved and what evidence is needed |

The feature's purpose and scope continue across later Proposals. Its original sources own their rules. This short navigation table does not maintain copies of those rules, check results or Task states.

## Run rule

The report domain owns this assumed rule, L1: a Run admitted by the service has a lifetime independent of a view observing it. Leaving the view releases that observer's resources and does not request cancellation. A Run, submission intent, result file and current observation are different facts.

The domain's native declaration must distinguish admission, known rejection and unknown submission outcome, then provide only the lookup/recovery operations its implementation can uphold. Current read authority governs disclosure of the original Run. An unavailable read does not establish a failed Run. The exact signature remains in that native declaration; this example supplies none and claims no provider exists.

## Export and return scenario

The complete-outcome owner maintains one scenario. Local criterion keys below support the references in later designs.

| Node | Criterion | Required owner and evidence |
| --- | --- | --- |
| Submit | C1: the response distinguishes admission, refusal and unknown outcome for the selected target | Submission contract; independent effect/receipt evidence for a lost response |
| Leave | C2: closing this view preserves the admitted Run under L1 | Run and observer owners; a real host observation, not a successful mock |
| Return | C3: under current permission, show the original Run's outcome or explain that it cannot currently be read | Read contract, navigation and view; identity and relevant result branches |
| Retrieve | C4: retrieve the completed readable result or explain the supported continuation when unavailable | Result retention/read contract and interface; actual result and authority evidence |

Retention duration, expiry behavior and required performance are open questions. They need decisions appropriate to the chosen release before C4 can be treated as an implementable settled promise. A design handoff can name these questions and their owners without inventing values.

## Result view

The result page refines C3, uses the original Run read operation and presents its result. It owns layout, labels, focus and return navigation. It must preserve the domain distinction between failed execution and an unavailable observation.

A prototype uses a named fixture for each selected view state, including unreadable, pending and completed results. Its question is whether a user understands the visible outcome and finds the next action. The prototype can demonstrate a treatment and retain feedback; simulated reads do not establish authorization or persistence. Its source notes refer to C3, the read contract and the view state design.

## Support and next questions

At this point the feature is designed in part. It is neither implemented nor verified. Retention and expiry remain substantive design work. A later actual support statement should name actor, scenario, channel, version and environment, link original evidence and retain gaps. It should not infer support from all documents having been written.

## Change: explicit cancellation

Suppose the user later needs to stop a no-longer-needed Run. Restore the original purpose and L1 before designing a Cancel button. Stopping observation, asking a Run to stop and deleting its result are different requests.

The proposed change adds explicit cancellation while retaining L1 and the return path. The actual Run owner must decide what it can guarantee: accepting a stop request may differ from confirmed termination, and completion may race with cancellation. A lost cancellation response leaves the original request's outcome unknown; the caller uses only recovery supported by that contract.

The rejected alternative in this example is automatic cancellation on closing the page. It would change L1 and C2. A browser close or network loss cannot reliably deliver a final request, so a page-unload callback cannot establish that promise. A product choosing this alternative would need a different, enforceable lifetime policy and changes to its consumers.

Record this bounded update set in the owning Proposal/Task:

| Source | Disposition and reason |
| --- | --- |
| Feature scope and scenario | Add explicit cancellation and its decisive race/unknown branches; retain C2/C3 |
| Run rule and native operation | Add the permitted cancellation request, observable outcomes and continuation; domain owner resolves feasibility |
| Authorization rule | Review whether existing scope permits this action; reuse it if applicable, otherwise change the original rule |
| Result view and prototype | Add action/pending/result expressions and representative fixtures; avoid equating request acceptance with stopped execution |
| Checks | Add the affected contract and host scenarios, including completion race and lost response; reuse unrelated checks only when their basis still applies |
| File-format transformation | Retain if its input/output guarantee and consumers are unaffected |
| Old observations | Preserve their original basis; do not relabel them as cancellation evidence |

This set follows changed meaning and actual dependencies. It is not a permanent second tracker or a demand to recheck every file in the repository. If implementation exposes an impossible guarantee, return that difference to the original product decision and affected designs.

## Partial delivery and shared rules

If CLI cancellation is later implemented and exercised while Web has only a design, the feature records those separate support facts. An explicitly CLI-only Delivery can close with its complete input/result/recovery evidence. A promised cross-channel result remains incomplete until its necessary paths and handover hold.

If another feature shares completed reports, the receiving scenario uses export, sharing and current read authority. Its complete-outcome owner keeps one scenario; participating features reference relevant criteria. The original retention rule remains with the file/result owner.

A later retention reduction must therefore examine both return and shared-report reading, including prior results and links. Cancellation and retention Proposals may proceed independently until they overlap on when a result exists or starts its retention period. Reconcile that shared rule at its owner; do not overwrite it from an old whole-document baseline.

## A small capability stays small

An internal pure function converting nonnegative milliseconds to duration text can keep units, rounding, input limits and examples beside its native declaration. It needs no product feature entry, prototype or parent planning chain solely because it is a callable capability.

Changing its private loop while preserving output uses the existing guarantee. Changing rounding changes meaning, so update the original rule, relevant examples/checks and known display consumers; a bounded Task can still be enough. If the product independently offers duration conversion as a CLI, its real actor, invocation, result and failure scenario establish the product feature. An API or CLI feature needs no invented graphical interface.
