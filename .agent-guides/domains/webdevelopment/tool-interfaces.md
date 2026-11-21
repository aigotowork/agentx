---
schema: scenario
description: Organize inspectors, workbenches and analysis pages around user questions, domain identity and useful evidence.
tags: [web, tools, information-architecture]
---

# Make a tool's model understandable and usable

First read [product and interface design requirements](../../scenarios/interface-design.md). Layout first, viewport efficiency, local disclosure and visual expression are required for this tool view, including small screens; reuse this context when already read.

Use for inspectors, run readers, data/configuration browsers and workbenches. A small tool still needs coherent information and behavior. Start from one task: what scope is being examined, what must be understood, what deserves attention, why, and what can be done next. Do not turn every API response into a card or every database table into navigation.

For product-wide organization of multiple subjects, navigation and management actions, use [management product design](../management-product-design/AGENTS.md). This guide remains the bounded method for a tool view's information and expression; a small inspector does not need a multi-page management shell.

## Select the model the user needs

| Question | Visible support |
| --- | --- |
| What scope and subject? | Project/object set, time window, version and active filters |
| Which object? | Human label and stable identity, selection and detail |
| How are objects related? | Named containment, dependency, production or comparison links |
| What changed or is pending? | Lifecycle, outcome, freshness and time as distinct dimensions |
| Why believe this summary? | Population/coverage, source, missing data and evidence entry |
| What does an action affect? | Exact object/scope, prerequisites, progress and actual result |

Keep current live state distinct from historical snapshots. “Finished”, “passed” and “recent” describe different facts. Failed reading is not an empty dataset; zero is not missing. Place a limitation where it affects the judgment, not in an unrelated help panel.

For model views, use [object cards and relationship graphs](../interaction-modeling/method.md#object-cards-and-relationship-graphs) to preserve property provenance and edge meaning. When a view claims coverage or completeness, use [independent sources and coverage](../interaction-modeling/sources-and-coverage.md) to establish its population, omissions and limits independently of the selected model.

Arrange overview → objects/groups → details → original evidence as levels of understanding, not four required pages. Preserve the task's necessary comparison on screen; defer long raw data and low-frequency settings. Collapsed areas retain useful summaries, counts and errors. Direct links/search may enter detail while still showing scope and return context.

## Choose expression by the question

| Task | First useful expression | Preserve |
| --- | --- | --- |
| Exact lookup, copying IDs, sorting or batch selection | Aligned table | Units, meaningful columns, selection and row identity |
| Trend or before/after comparison | Shared-axis plot or explicit paired view | Window, baseline, scales and missing samples |
| Distribution and outliers | Distribution plot plus inspectable objects | Population, bins/units and route to the underlying values |
| Containment | Tree or grouped list | Parent meaning, partial loading and stable object links |
| Dependency or flow | Bounded graph or sequence | Edge meaning/direction, omitted nodes and relation to action order |
| A few simple facts | Text or compact key/value structure | Clear labels and context |

Use graphics where position, length or relationships reduce interpretation effort; do not add decorative gauges or an unreadable complete graph. Keep a readable summary/data alternative and keyboard access. Colors need textual or shape meaning. A dense aligned table can be clearer than tall cards requiring memory across scrolls.

Filtering narrows scope, sorting supports comparison, disclosure exposes supporting detail, split views retain context and tabs switch perspectives of the same subject. Do not split information needed for one comparison across tabs. Preserve useful filters, selection and drafts on return. Use stable addresses for revisitable scopes/objects. At narrow width, keep the core workspace together through layout adaptation and local disclosure. Use separate navigation only for a substantially different task/context, preserving the subject and return relationship.

## Design meaningful control and precision

Derive the capability depth from task → object → property → operation → input precision → result → recovery. The useful layers may include collection search/selection/batch actions, object operations, property editing, relationships/order, preview/apply and return/undo. A checkbox needs no invented management subsystem, and a read-only investigation can be complete without mutation.

For each required capability, identify stable semantic action, exact target/use-site/version and scope, input source, expected outcome and recovery. One handler may expose several variants; several controls may expose one capability. Distinguish capability kinds, entry instances, user steps and DOM depth. Do not add dummy or duplicate buttons to satisfy a count.

For numeric work, expose actual value, units, meaningful step, bounds, reset and a keyboard path; dragging may supplement precise input. Displayed decimal places do not establish saved precision. A frame-based edit needs a declared frame/time conversion that survives round trips. For batch work, show selected scope, eligibility, mixed values and partial results rather than applying a hidden global default.

Keep frequent operations beside their object or work surface, with secondary controls disclosed locally. Preserve the image/canvas/data and the facts needed to compare while adjusting. Dense layouts must keep readable text, usable targets, focus and accessible names; shrinking everything is not information efficiency. Separate a substantially independent task when its return contract is explicit.

Use the focused [tool experience review](../product-design/scenarios/tool-experience-review.md) for complete task, cognition, domain fit and return behavior. DOM/source/contrast measures can support a declared diagnosis. Thresholds are task-specific configuration, not industry laws; button counts, component counts, token ratios and a passing lint cannot establish quality. Keep actual engine/SDK support and tool prerequisites with their owners.

## Close the action and evidence loop

For the chosen path, connect entry, filter/search, selection, interpretation, action where applicable, result and return. Bind actions to stable domain identity, not the highlighted row index. A submitted request remains pending until its actual contract establishes an effect; unknown results use the original recovery contract. A Retry button cannot manufacture safe replay.

Check representative dense data, one meaningful exception, no matches, stale/partial data, unknown action result and keyboard/narrow layout where they affect the task. Observe whether someone can explain scope, distinguish states, find the supporting objects and recover context. If no user was observed, label the work as a design walkthrough. Compare revisions with the same task/data and judge correctness of understanding as well as elapsed time.

Use [experience design](experience-design.md), [interaction/state](interaction-and-state.md) and [observation](observation.md) for their respective decisions. Stop when the bounded task is understandable and operable with adequate evidence; a new dashboard framework or brand system is not a prerequisite.
