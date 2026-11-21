---
schema: scenario
description: Review complete tool workflows for domain fit, learnability, precision, attention and recoverable continuity with bounded evidence.
tags: [product-design, tools, evaluation, cognition, continuity]
---

# Review how a tool serves the user's work

Use for a workbench, editor, resource manager or substantial cross-region change when the question is whether users can understand, control and finish their work. Read [interface requirements](../../../scenarios/interface-design.md) first. A bounded color/label change reuses the existing scenario and checks affected states; it does not require a new task analysis or whole-product review.

Use canonical scenarios and success criteria, and retain results in the existing design/review/Task. [Evaluation](../evaluation.md) owns evidence and finding disposition; [tool interfaces](../../webdevelopment/tool-interfaces.md) owns view organization, [interaction/state](../../webdevelopment/interaction-and-state.md) owns UI behavior, and [navigation/recovery](../../webdevelopment/navigation-and-recovery.md) owns return contracts. Use [interaction modeling](../../interaction-modeling/AGENTS.md) when cross-source bindings or coverage need explicit analysis. This review composes those methods rather than creating another requirements source.

The [working template](tool-experience/review-template.md) is optional and adaptable. The [constructed example](tool-experience/worked-review.md) demonstrates scope and return failures that control counts miss. Expert reasoning is useful without participants, but is not observed user behavior.

## Select the professional lens

| Method | Use it to decide | Limit |
| --- | --- | --- |
| Hierarchical task analysis (HTA) | Decompose goals into necessary subgoals; state sequence, conditions, loops and stopping rules | A menu/click/component tree is not a task model. Without research input, the task model is a hypothesis |
| Cognitive walkthrough | At key steps, predict intention, discovery, action–effect association and comprehension of feedback under a stated knowledge assumption | Expert prediction, especially of learnability; not participant behavior or skilled-user efficiency data |
| Usability heuristics | Diagnose status visibility, domain language, consistency, control, error prevention, recognition, efficiency and recovery | Diagnostic lenses, not ten quality scores; simplicity does not mean deleting necessary precise control |
| External cognition/recognition | Expose identities, differences, parameters and pending work that otherwise must be remembered | Text length or control count cannot calculate cognitive load; more visible material can also increase search interference |
| Related-information integration | Identify facts that must be compared but force repeated searching, remembering and integration | Split-attention research on instruction does not prove all page changes are wrong or all single-page designs better |
| Interruption and goal recovery | Inspect task cues and unfinished work before leaving, during a switch and on return | No fixed time penalty applies to every switch; measure only a defined task and population |
| Visual/accessibility review | Inspect hierarchy, alignment, contrast, states, focus and operation with representative data/input | A screenshot, token check or personal preference alone cannot establish usability, beauty or compliance |

Usually one task-path table can expose several concerns. Deepen the risky joins rather than launching a separate process for every lens. No extra reviewer, workshop, executor or score is required.

## Fix the goal, scope and evidence

Record actor and experience assumptions, scenario/criterion, initial data and authority, object/version, expected outcome, candidate/build, environment, viewport and input method. Distinguish the user's words, chosen requirements, native contract facts and reviewer assumptions. Select a complete main path and the interruption/failure/return branch most likely to invalidate its result. Add branches when the promise warrants them.

Ask what object must reach what state, and how the person can know it happened. “Apply the reviewed change to these selected uses at this baseline and preserve unrelated uses” is more useful than “has Apply.” First use, frequent repetition, long editing and occasional administration may need different paths.

Record existing strengths and their mechanisms as conditions to preserve. Do not manufacture a defect for every API or heuristic. Classify selected promises as supported, gap, contract conflict, undecided, out of scope or unverified, with evidence. Separately distinguish prototype placeholder, target extension, implementation defect and missing evidence. Resolve unclear expectations before declaring a defect.

## Expand the outcome into a complete task

Decompose only to subgoals with judgeable results. For example: locate → compare → choose scope → adjust/preview → decide → apply → verify/continue. Express plans such as “compare before deciding,” “repeat only failed items” and “stop when no change is wanted.” Do not assume every task is a linear wizard.

| At each meaningful node | Inspect |
| --- | --- |
| Intention/judgment | Why this step exists; required facts, comparisons and constraints; whether they are available together |
| Object/identity | Native object, version, reference/use site and role; definition versus instance, work versus execution attempt |
| Operation/scope | One property/object, selected set, all references or global default; visible scope before action |
| Input/control | Needed precision, batch behavior, constraints, reset and preview; whether the domain can store and round-trip the displayed precision |
| Effect/result | Preview, draft save, dispatch and result adoption change different things; where to verify difference and provenance |
| Capability/responsibility | Native engine/SDK, product adapter, UI and any participating agent; provider for required facts and persistence |
| Exception/continuation | Refusal, partial completion, unknown effect, stale baseline and lost permission; whether continuing repeats completed work |

Trace tasks to real entries and capability sources. One task can call several operations; one internal operation may need no direct UI. A type or UI name does not create a supported task, session or version lifecycle. If input/identity supply or Mock support is absent, use the existing [contract](../../../scenarios/contract-modeling.md) or [Mock method](../../../scenarios/mock-provisioning.md) and return the gap to that owner.

Canceling waiting, canceling execution and undoing an applied result differ. Labels and recovery must match native effects. Reconcile an unknown result through its original operation identity before any retry that could repeat effects. For batches distinguish completed, failed and uncertain items; do not resubmit the entire set by default.

## Walk through cognition at the critical steps

Start from the actual visible state, not an entry already known to the reviewer. State assumed user knowledge. For each critical step, answer with specific interface evidence:

1. Would the person form the intention to achieve this subgoal now?
2. Can they discover, reach and use the action? Presence in DOM or documentation is insufficient.
3. Do name, icon, position, object and scope connect that action to the desired effect?
4. Does feedback explain what happened, progress toward the goal, and the next or recovery action?

Keep facts and inference separate: “no selected-count/scope cue is shown” is a visible observation; “people may expect all references to change” is an expert hypothesis. Record prior instruction or assistance; a guided expert attempt is not independent first-use discovery. For skilled repeated work, also inspect shortcuts, direct manipulation, batching and appropriate retained settings.

## Review attention, switching and continuity

Keep facts and actions for one goal/object in its workspace through selection, local toolbars, inspectors and disclosure. Independent search, organization administration or unrelated configuration may have a separate page. Judge task boundaries, comparison needs and return cost; a dense single page can still fail through poor hierarchy or competing overlays.

For a consequential switch, define only the return state that the promise needs:

| Context | Return decision |
| --- | --- |
| Work identity/version | Same identity; explain changed, missing or inaccessible objects and provide valid re-entry |
| Selection/scope/location | Selected set, active item, filters/sort, scroll/canvas/play position; avoid a silent new default target |
| Draft/edit mode | Saved/unsaved, valid/conflicting, read/edit; name holder and lifetime rather than relying on browser memory |
| Comparison | Pinned baseline, candidate, difference and adopted choice; mutable “latest” cannot silently replace a baseline |
| Work/results | Background operation identity, completed/pending/failed/uncertain items and route to original result |
| Focus/task cues | Useful focus after disclosure and a visible reminder of current step and remaining work |

Recovery is not unconditional restoration. Deletion, revoked access, invalid version or changed dependencies require revalidation/reselection. Preserve recoverable draft and original scope, and explain why applying is blocked. Never silently apply stale work to a default object. Cross-refresh or cross-device persistence is required only when promised and supported.

Select applicable interruptions before submission, while waiting, after partial completion or following version change. Observe concrete repeated search, selection, input or comparison reconstruction on return. Do not invent frequency, time cost or an attention score. If measuring duration, define participants, object, start/end events and assistance.

Inspect whether current object/goal stays clear, selection differs from keyboard focus, parallel progress is relevant, errors remain attached to their objects and notifications steal editing attention. Scope summaries, differences and pending-work cues can provide external memory; long explanations and repeated confirmation are not default remedies.

When information must be integrated, consider adjacency, paired views, synchronized emphasis, retained summaries or local disclosure. Confirm it serves the same judgment before adding more content. Do not place unrelated information together merely to avoid navigation.

## Review precision and visual operation

Precision must serve the domain task. Check numeric value/unit/step/bounds, meaningful keyboard or direct control, reset and actual stored round trip; a number input or finely formatted display is insufficient. If the model stores whole seconds, a millisecond display does not provide millisecond control. A frame-based task may need integer frames and lossless conversions.

For selected sets, distinguish common and mixed values, locked/ineligible members and item/use-site/global scope. Verify exact changed items and partial outcomes. Check preview/apply/undo against actual effects. Do not require every small component or read-only research view to provide batch editing, history or CRUD.

Keep task, data and viewport comparable when reviewing treatments. Inspect primary work surface and regional proportions; useful density and grouping by object/stage/frequency; typography, aligned numbers/units and readable full labels; consistent surfaces, icons and semantic states; hover/focus/selected/disabled/loading/error/pending/drag behavior; target size, contrast, zoom, narrow layout and keyboard/touch alternatives. Follow [Web review](../../webdevelopment/review.md) and the [design system](../../webdevelopment/design-system.md) for implementation concerns.

Do not increase density by shrinking text/hit areas, adding repeated buttons, flattening every advanced action or removing useful recovery guidance. Preserve semantic HTML inside components; anonymous wrappers or more component types cannot establish better responsibility boundaries. Token use supports coherent expression but does not prove hierarchy or beauty.

Before reading an image, name the task/design question. Afterward state what its content/layout/actions/state show and the observation's limits. Keep a local finding concrete, such as “scope and Apply can be checked together.” Focus return, hidden calls and temporal behavior require interaction evidence. Do not use a sparse empty state to prove an improvement over a dense real task.

## Turn findings into owned decisions

For each substantive finding retain criterion/trigger, version/state/environment, observation and locator, expectation source, interpretation, consequence, responsible object/layer, proposal and decisive recheck. Keep severity and evidence certainty independent. Prioritize observed blockers, data loss and contract violations, then risks with an explicit mechanism; record preference and unknown frequency separately. Merge symptoms at their common cause.

State what the change enables, which strengths/promises it preserves, affected state/entry, alternatives/tradeoffs and validation. Use [evaluation's disposition method](../evaluation.md#decide-what-each-finding-calls-for) to distinguish defect, scope extension, unresolved design and missing evidence. Retain original observations and link later evidence. Proposed, selected, implemented and exercised are separate results.

### Let checks establish only their supported part

| Evidence | Useful contribution | Limit |
| --- | --- | --- |
| Scenario/native contract | Expected outcome, identity, scope, authority, state and responsibility | Does not establish discovery or delivery |
| Static source/component/CSS check | Call/state-retention clues, responsibility, tokens and state definitions | Does not execute behavior or measure cognition |
| DOM measurement and explicit path assertions | Visible/operable entries, sampled depth/precision and selected result/return states | Selected configurations only; no intent, aesthetic or complete-coverage verdict |
| Rendered view and interaction walkthrough | Visible hierarchy, feedback, continuity and recovery | Expert prior knowledge; simulated service is not real persistence/effect evidence |
| User task observation/interview | Discovery, misunderstanding, behavior and reported feelings under stated conditions | Keep observation/report/help/inference distinct; small samples do not represent everyone |

When automation is useful, derive assertions from the task table and use an actually installed tool's supported contract. To establish continuity, edit/select/compare → leave → return → assert in the same path/context. Two independently reset snapshots cannot prove retention. Verify persistence/conflict/unknown effects through the actual host/service boundary.

Counts can describe unique capabilities, entry instances, required actions, sampled discovery depth and precise parameters separately. User steps, DOM nesting, HTA depth and component depth are different units. Declare denominator, sampling, aliases, missing data and unvisited states. The shallowest observed path is not necessarily the shortest product path. Team budgets need scenario reasons; no universal button minimum, component ratio, attention total or fixed weighted quality score follows from this method. Do not lower a real requirement or add placeholders to make a report pass.

Stop when the selected design decision has enough bounded evidence, findings have dispositions and owners, and remaining questions have concrete investigations. A review can finish without implementing every proposed future capability. Review completion, passing lint and a pleasing static view do not establish delivered experience or user benefit.

## Professional references and their scope

The method combines [task analysis](https://www.nngroup.com/articles/task-analysis/), [cognitive walkthroughs](https://www.nngroup.com/articles/cognitive-walkthroughs/), [usability heuristics](https://www.nngroup.com/articles/ten-usability-heuristics/) and [recognition versus recall](https://www.nngroup.com/articles/recognition-and-recall/). The four questions above are a simplified expert walkthrough, not a claim to reproduce a full research workshop.

Research background includes Scaife and Rogers, [External cognition](https://doi.org/10.1006/ijhc.1996.0048); Altmann and Trafton, [Memory for goals](https://doi.org/10.1207/s15516709cog2601_2); and Chandler and Sweller, [Split-attention in instructional design](https://doi.org/10.1111/j.2044-8279.1992.tb01017.x). These references motivate questions about representations, pending goals and related information. They do not justify tool-specific effect sizes, fixed switching penalties or universal page rules; the workflow recommendations require task-specific judgment. Apply accessibility criteria, such as [Focus Not Obscured](https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html), with their conditions and exceptions; this review is not a compliance certification.
