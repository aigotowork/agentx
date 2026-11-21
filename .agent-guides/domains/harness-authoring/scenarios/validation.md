---
schema: scenario
description: Validate guidance for user value, Agent usability, compatibility and delivery integrity
tags: [harness-authoring, validation]
---

# Validate a guidance change

Use after changing behavior, routes or required resources. Select checks for the actual change; a wording correction needs less than a new method. Reuse the owning [scenario design](scenario-design.md) and its cases. This method requires neither a separate reviewer nor delegation.

## Test the two perspectives

| Perspective | Walkthrough |
| --- | --- |
| User | Start with the intended task and expected result. Can the method help complete it, preserve requirements and handle relevant failures? Does its output have a use beyond satisfying its own checklist? |
| Agent | Start with only the real task and reachable context. Can the Agent detect applicability, select or combine methods, obtain prerequisites, decide actions and judge completion? |

Choose a normal case and the cases that expose the change's main risks: incomplete inputs, a nearby excluded task, a relevant exception, or several simultaneous conditions. Do not require a fixed case count for every edit.

For the Agent view, examine missed and false triggers, vague rules that do not distinguish actions, misleading examples, conflicting instructions, hidden prerequisites, duplicated context and excessive reading or interpretation. Explain the mechanism of an expected benefit or harm. Do not supply missing knowledge from the author's memory to make a case pass.

When changing coordination or evidence rules, also walk through the choices that could multiply work:

| Situation | Expected boundary |
| --- | --- |
| One qualified implementer can complete a change across several files | Keep the cohesive assignment; do not add a Controller, review Agent or per-file handoff |
| Independent contributions can overlap usefully | Permit parallel execution within authority; do not turn the single-implementer default into mandatory serialization |
| A designated executor reports a required check passed | Consume it for the assigned decision; do not initiate duplicate audits, fresh receipts or acknowledgement loops |
| A relevant input changes or evidence contradicts the claimed result | Reopen affected checks and preserve unrelated established results; do not waive actual integration or cutover requirements |

Use these as conditional walkthroughs for the changed rules, not a new suite or report required of every consuming task. They can reveal ambiguous instructions; they do not prove runtime scheduling or model compliance.

If a rule is too general, derive a concrete condition and action or fix its route. If it is too narrow, revise its assumptions or limits. If the path is burdensome, remove duplication or improve selective reading without hiding required constraints. Record substantive findings and their resolution in the existing design/task.

## Check the affected surfaces

- Verify agreement with current authority, artifact ownership, subject contracts and related methods.
- Check changed paths, anchors and metadata with available tools; distinguish examples from actual references.
- If publication changed, verify required resources in the consuming layout. Author-checkout links alone do not establish portable guidance.
- Use an actual bounded Agent task when effectiveness is materially uncertain and suitable execution is authorized. Keep input, observed result, omissions and relevant context/cost observations; do not infer behavioral success from a valid document.

## State what the evidence establishes

| Evidence | Supports | Does not establish |
| --- | --- | --- |
| Design reasoning | A plausible mechanism and explicit assumptions | Observed effectiveness |
| Static case walkthrough | Interpretable choices and visible coverage under stated inputs | That a model always follows the method |
| Link, metadata or installation check | Specific structural and delivery properties | Correct professional judgments or task outcomes |
| Actual use | Observed behavior in that model, context and task | Universal reliability or cost reduction |

Stop when the changed scope has adequate evidence and known limits are recorded. Do not rerun passed checks unless later changes could affect them or an explicit requirement demands it. New problems outside the current outcome return to their owning discussion or task; successful validation does not authorize unrelated implementation or publication.
