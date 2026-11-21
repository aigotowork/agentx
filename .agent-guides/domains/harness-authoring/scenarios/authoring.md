---
schema: scenario
description: Create or revise owned principles, Role, Scenario and Skill guidance
tags: [harness-authoring, guidance]
---

# Author or revise guidance

Use when creating or changing governance instructions. For new or materially changed behavior, first establish the [scenario design](scenario-design.md) in its relevant sketch/design. Reuse valid existing decisions for bounded revisions. Apply [writing](../writing.md) to the changed content.

## Choose the useful change

After understanding the target behavior and affected sources, prefer the first adequate option:

1. Existing guidance covers the need: use it without adding another rule.
2. The method exists but is hard to find: repair its route.
3. A condition, boundary or example is missing: amend its owning section.
4. A distinct situation needs its own method: add a focused scenario.
5. Several scenarios share stable concepts and responsibilities: organize a domain.

Adequacy includes the actual requirements, correct ownership and usable context. The selected change includes necessary related edits; this preference does not require the smallest diff at the expense of coherence. Existing authorization remains sufficient for work within its scope.

Do not promote a temporary precaution or an Agent's preferred proof into a permanent workflow default merely because it appeared in a prior task or decision. A recurring requirement needs a concrete failure mechanism or an explicit user/product requirement, an applicable situation and a stopping condition. Remove superseded rules and local copies when moving the method to its shared owner; do not accumulate contradictory safeguards.

## Fit the content to its subject

| Subject | Owns | Keep elsewhere |
| --- | --- | --- |
| Principle | A stable constraint or judgment, with useful conditions or deductions | Detailed domain procedures |
| Role | Purpose, responsibilities, authority boundaries and relevant scenario routes | Task execution state and duplicated methods |
| Scenario | Situation-specific choices, method, useful intermediate results and completion boundary | Other scenarios' authoritative rules |
| Skill | A capability reusable outside this guidance system, its prerequisites and native entry | Project-specific orchestration and author history |

Usually keep a Role in one file. A Scenario may address related situations and compose Skills or other methods. A Skill preserves its native format and keeps required reading/resources self-contained where possible; declare unavoidable dependencies. Directory placement alone does not register it with an Agent platform.

## Write and connect

Inspect existing methods and routes for overlap and conflict. Keep each explanation at one meaningful owner. When material is adapted, use [adaptation](adaptation.md); when relations or paths change, use [composition](composition.md).

Write enough for the consuming Agent to recognize the situation, act within the stated scope and judge the result. Preserve the necessary input, constraints, failure behavior and stopping condition without forcing identical headings onto every guide. Use subject-specific intermediate questions and results, linked to [method working records](../../../scenarios/project-context.md#make-method-execution-explicit).

Use [shared metadata](../../../refs/xschema/common-schema.md), including required Scenario tags, and preserve native Skill metadata. Metadata supports discovery; keep methods and conditions in the body.

Check the affected [validation](validation.md) concerns after edits. Update the design when decisions changed, and keep execution evidence in the owning task. Follow the project's existing publication process once content and required resources are ready. Stop when the need is resolved and the real entry reaches a usable method.
