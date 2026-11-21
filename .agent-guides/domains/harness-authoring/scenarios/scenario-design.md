---
schema: scenario
description: Establish the situation, rationale, effectiveness and user/Agent fit before designing guidance
tags: [harness-authoring, scenario-design]
---

# Design the guidance situation

Use when creating guidance or materially changing its scope, method, workflow or behavior. This applies to Skills, principles, Roles, Scenarios and entry content. Start from the work being guided, not the proposed document form.

## Record the design basis

Before finalizing the content, record the following in the owner's relevant `.sketches/` or design area, following [placement rules](../../../documentation.md#artifact-placement-rules). Update an existing record where possible. One section or table may cover several related guides; a separate file per guide is unnecessary.

| Explain | Make concrete |
| --- | --- |
| Situation and scope | Who is doing what, when this method applies, the starting state/input, desired outcome and excluded situations |
| Motivation | The real problem or recurring decision; why existing behavior or guidance is insufficient; what should change |
| Basis | The domain methodology, principles or evidence used, their assumptions and how they support this method; distinguish sourced facts from hypotheses |
| Workflow and effectiveness | If there is a flow, the meaningful decisions, inputs, intermediate results and exit conditions; why each important transition helps reach the outcome. For a rule without a flow, explain the judgment it changes |
| Conditions and limits | Required context, tools, authority, dependencies, exceptions, failure conditions and cases where the method does not apply |
| User perspective | The user's actual task and expected result; whether the content helps accomplish it without unexplained prerequisites or unwanted scope |
| Agent perspective | What the Agent will actually see, how it detects applicability, finds needed information, acts and knows when to stop; assess misleading, ineffective or burdensome guidance |
| Evidence and uncertainty | What was inspected or tried, what is only reasoned, unresolved assumptions and the smallest useful way to test them |

These are required design concerns, not required headings or a universal form. Link existing answers instead of copying them. A cosmetic edit reuses a still-valid design. If relevant reasoning is missing when a material change is made, fill that gap within the affected scope; do not launch an unrelated retrospective documentation project.

Make the basis reachable from the author task, design or local author entry. Promote stable decisions to their owner when appropriate. Consuming Agents should not have to load the full research history to execute the method.

## Walk through the user's situation

Use a concrete request and expected outcome. Identify what the user supplies, which choices remain with them and what the method must produce. Follow the proposed flow to its observable result, including relevant failures or incomplete inputs.

Check whether a person using the result can accomplish the original goal. A completed checklist, a shorter response or a new document is insufficient if the task remains unsolved. Preserve the user's actual requirements and existing authority; do not add confirmation steps merely to make the workflow look controlled.

## Walk through the Agent's visible context

Start at the actual entry point with the task input. List the essential information already visible, what is reached conditionally and what is unavailable. Do not silently supply facts from the author's memory or an unlinked design.

| Failure to examine | Useful question |
| --- | --- |
| Missed trigger | Would this real request cause the Agent to select the guide? Is the route specific enough to recognize? |
| False trigger | Would a nearby but excluded task load or apply it unnecessarily? |
| Ineffective instruction | Does the text distinguish a concrete next action, or merely say “be clear”, “use judgment” or “read relevant material”? |
| Wrong action | Can a broad rule, example or compressed sentence remove a requirement, imply authority, invert order or conflict with another method? |
| Context burden | Are always-loaded details, repeated sources, deep routes or circular reading requirements adding cost without changing the decision? |
| Cognitive burden | Must the Agent decode invented abbreviations, infer missing conditions, reconcile inconsistent terms or reconstruct a hidden workflow? |
| Unavailable prerequisite | Does execution depend on an absent tool, inaccessible reference or source-checkout path? What can still be done accurately? |

User and Agent are perspectives, not necessarily separate actors. User fit tests the value of the result; Agent fit tests whether the presented information can lead to it.

## Resolve the design before polishing

Use a typical case and whichever boundary or counterexample exposes the main uncertainty. Derive the condition and action that make the method useful. When evidence is limited, record an effectiveness hypothesis and its limits rather than inventing proof or overstating a borrowed method's results.

Leave with a bounded situation, a plausible and explainable method, known limits and a usable reading/action path. Then apply [authoring](authoring.md). If the need is already covered, repairing a route or clarifying an owned rule may be the complete outcome. Use [validation](validation.md) on the resulting content; do not repeat the design exercise for unchanged decisions.
