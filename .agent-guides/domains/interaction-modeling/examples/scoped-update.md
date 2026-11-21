---
schema: document
description: Constructed manual interaction slice illustrating use-site scope, partial effects, unknown outcomes and tutorial recovery.
---

# Worked slice: apply a reusable style to one use site

This is a constructed example, not an observed product, executed check or user study. Names below are illustrative native references to replace with a consumer's actual sources. No checker or service is required to reason through it.

Assume the canonical criterion promises: choose an existing style candidate, inspect its effect, and apply it only to selected use site `U7` at revision `R4`; preserve the shared style definition and other uses. Related resource management can be opened and the comparison resumed. The domain owner supports version-checked application and querying an operation by its original identity.

## Sources and object roles

| Axis | Illustrative original authority | Model reference |
| --- | --- | --- |
| Scenario | Criterion “apply selected style to one use” | Criterion and normal/conflict/unknown branches |
| Domain | Style definition, use-site/revision, preview and operation receipt declarations | Candidate `C2`, use `U7`, baseline `R4`, preview `P9`, operation `O3` |
| UI | Candidate picker, paired comparison, scoped Apply control and result panel | Selection, comparison, input mapping, feedback and return |

`U7 references StyleA` is an association with an explicit reference key; it does not imply applying to `U7` mutates `StyleA`. A hypothetical edge to a publishing service remains unverified and cannot establish a dependency. The dependency that matters is conditional: version-checked apply needs the original `U7/R4` and validated preview. A schema for `revision` does not supply its value; the authoritative read must do that.

## State and bindings

Initial slice: `D` contains `U7/R4` and `C2`; `U` contains selected `U7`, comparison baseline `R4`, draft and return position; `A` permits applying to `U7`; `E` contains no submitted operation.

| Step | Control/action and provider | Result/next state | Preserved facts |
| --- | --- | --- | --- |
| Inspect candidate | Open `C2` in the current comparison | Candidate and baseline visibly identified | Original selection and draft |
| Prepare preview | Preview input from `C2` plus authoritative `U7/R4` | `P9` returned, then actually displayed for review | No committed change |
| Visit resources, if needed | Navigate with task-owned return context | Re-read original identities on return; invalidate if stale | Draft, comparison, scope, position and useful focus |
| Apply | Submit `P9`, `U7/R4`, original operation identity `O3` | Accepted → observe completion, or refusal/conflict/unknown | Shared definition and all other uses |
| Verify | Read `O3` and resulting `U7` revision | Visible effect and next action tied to the same work | Unrelated facts remain unchanged |

Creating a reusable resource before application would act on a different object role. That is legal if the native flow promises it; do not force its facet to equal `U7`. If creation succeeds and apply conflicts, report the retained created resource and unapplied use as a partial result.

## Decisive counterexamples and finite limits

| Constructed trace | Judgment and owner |
| --- | --- |
| Compare `U7/R4` → active global selection changes to `U8` → Apply reads global selection | Binding/scope violation: pass the selected use identity explicitly; UI/adapter owner |
| Domain input declares `R4`, but no authoritative read or prior result provides it | Supply gap despite valid type/example value; domain/client owner |
| Compare `R4` → remote update to `R5` → submit `R4` | Contractually reject/resolve conflict; do not silently substitute `R5` |
| Submit `O3` → reply lost → reopen → submit a new operation | Unsafe recovery unless native contract establishes it; query/reconcile original `O3`, retain unknown if unavailable |
| Preview service returns `P9` → Apply immediately without displaying it | Computation exists, but user review promise is unfulfilled; product/UI owner |

A hypothetical finite check might use two use sites, two revisions, allowed/denied permission, and completion/conflict/lost-reply events with a bounded path length. Record actual chosen limits, frontier, pruning and results if executed; this example supplies no run. Never merge `U7` and `U8` if selected identity is the property under test. A successful path would establish reachability, not inevitable completion.

## Independent coverage example

Suppose independent sources reveal 3 criteria, 4 domain variants and 6 control occurrences normalized to 4 semantic actions. Join and report those units separately. A dynamic factory with unknown action names remains a discovery site of unknown cardinality. It cannot be omitted to report complete discovery.

The scoped-apply criterion can be catalogued, bound and modelled while UI execution remains absent. An internal domain variant can have an explicit no-UI disposition. An unmatched control can suggest a candidate criterion, but only its product owner can adopt it. Full classification closes neither implementation nor user-validation obligations.

## Recoverable demo

A lesson can teach inspect → preview → review → explicitly apply → verify. Stable targets include `U7` and the relevant control role, not a row index. Completion of preview requires visible `P9`; completion of apply requires the observed native result.

On reload after dispatch but before checkpoint, restore `O3`, selected scope and comparison, then query the original operation. Do not dispatch again. If its result is unavailable, show unknown and the recovery entry. A late callback captured for an earlier run/step cannot advance the resumed lesson. A changed definition or invalid `R4` requires explicit re-entry/reconciliation, not transplanting old progress.

These walkthroughs explain the method's judgments. Actual checker, UI/Mock, real-service and participant evidence must be obtained and reported separately in the consuming task.
