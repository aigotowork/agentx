---
schema: scenario
description: Derive a collaborative logo workbench from design tasks, shared context, role boundaries, iteration continuity and observable outcomes
tags: [logo, collaboration, workbench, interaction]
---

# Collaborative experience and workbench design

Use when designing the logo workbench, connecting chat and visual tools, defining workflow interactions, or reviewing whether a prototype actually supports the design method. It owns the mapping from professional collaboration to product behavior. The [conversation method](conversation.md) owns dialogue, [presentation](presentation.md) owns comparison views, [records](records.md) owns semantic facts, and [tool selection](../tooling.md) owns capability and source/export boundaries. Native implementation contracts stay with the consuming tool owner.

For actual product work, compose [interface requirements](../../../scenarios/interface-design.md), [product design](../../product-design/AGENTS.md) and [interaction/state guidance](../../webdevelopment/interaction-and-state.md). Use [management product design](../../management-product-design/AGENTS.md) when scope includes multiple work subjects and navigation. These methods own general product decisions; this guide adds identity-specific collaboration. It is a design method for a requested workbench, not a supplied application. Ordinary logo rounds use the same records in chat and local files without implementing a workspace.

## Design the collaboration, then its interface

Start with a representative user task, current subject/version, decision, required evidence and next action. Derive the work area and interaction from those answers. Do not turn the method's directory, a record schema or a list of tool capabilities into a page of cards.

The governing relationship is:

`brand problem → collaborative decision → required context → actions and outcomes → workspace expression → evidence`

A workspace is useful when the user can advance that relationship without reconstructing its context in memory. A static gallery can legitimately support a bounded review task, but it is not evidence that annotation, agent collaboration or recovery works.

## Principles and their local consequences

| Principle | Consequence for this workbench | Boundary |
| --- | --- | --- |
| Recognition and external representation | Make actual alternatives, retained cues and comparison baselines visible; offer relevant professional language | Do not display the entire vocabulary or force annotation of every object |
| Direct work on the subject | Keep selection, comparison and annotation attached to the candidate/reference/version in view | A region annotation is not automatically a request to edit all instances of that shape |
| Shared context | Preserve user statements, agent interpretations and proposed changes as attributable, inspectable facts | An interpretation is not a confirmed user preference |
| Mixed initiative | Let the user steer fit and preference while the agent proposes, diagnoses and makes; tools preserve the relationship | Neither mandatory user micromanagement nor hidden autonomous aesthetic decisions |
| Reversible exploration | Keep sources, branches, baselines and editable feedback; allow an experiment without erasing earlier work | Undoing a view change does not cancel an external generation operation |
| State visibility | Distinguish draft/save state, execution, asset availability, evidence and design choice | A single completion badge cannot represent all of these |
| Continuity | Preserve the current question, subject and unfinished expression across related views and interruptions | Recovery follows the real persistence and operation contract, not an assumed autosave |
| Progressive disclosure | Expose the facts and actions for the current decision; reveal low-frequency detail locally | Required constraints, stale input and missing results cannot be hidden as advanced information |

These principles are design mechanisms with conditions, not a guarantee of better outcomes. The [source basis](../research/sources.md) identifies inspected material and limits; the relationships below explain how to apply them without loading author research.

## Assign responsibilities without transferring the design burden

The user supplies context, reactions, preferences and acceptance scope. The agent abstracts intent, researches, creates alternatives, diagnoses form and craft, explains trade-offs and produces/verifies artifacts. The workbench preserves their common subjects, versions and decisions and supports actions at the appropriate point. Renderers and generators execute declared inputs and report actual outputs.

Do not require a user to identify a technical defect before recording dissatisfaction. Do not let a workbench infer that selecting a descriptor endorses the whole candidate. Do not ask the user to approve every internal plan when the work is already authorized. Make interpretations correctable and material input differences visible instead.

## Maintain two connected loops

The **design loop** refines understanding: question → alternatives/evidence → reaction → interpretation → next design question or decision. Its output can be a reference study or a revised brief, not only generated artwork.

The **execution loop** carries out a chosen activity: declared input snapshot → actual operation → results/partial failure → usable artifacts and evidence. Completing an operation does not select a logo, prove its quality or resolve the design question automatically.

Use a round or iteration as the context for a design question, with the relationships defined in [records](records.md). Execution requests belong to their initiating plan, brief and source versions. A round can include several operations or none. View changes do not advance the round's meaning by themselves.

A user's new opinion can be recorded while work is active. Preserve whether it is part of that operation's input, a later amendment or a future round. Late results return to their initiating context; do not attach them to the currently highlighted candidate. Reopening a concept or changing a brief can affect several conclusions without deleting history.

## Separate work state from presentation state

| Fact | Appropriate owner | Visible effect |
| --- | --- | --- |
| Confirmed brief, scoped preference and decisions | Project records | Relevant retained constraints and context |
| Agent interpretation and plan revision | Attributed interpretation/round records | Concise, correctable account of the proposed change |
| User's unfinished annotation | Its editing/draft context | Actual unsaved, saving, saved or failed state, as supported |
| Feedback consumed by a running operation | Operation input snapshot | Which submitted opinions this result can address |
| Running/failed/unknown operation | The actual execution capability | Status and legitimate recovery near its affected work |
| Asset version and derivation | Canonical asset records | The right source, variant, comparison and provenance |
| Hover, open panel or transient zoom | Local view context | Disclosure and navigation without new design decisions |
| Addressable selected subject or comparison | Workspace/navigation state where needed | Direct entry, restoration and useful return context |

Choose the commit behavior of each control explicitly. A direct “keep this contour” action may save immediately; a text editor may retain a draft until submission. The UI must distinguish local input, durably recorded feedback and feedback included in an activity. Do not add a Save button to every interaction or a new confirmation step to every refinement.

## Derive the workspace from decisions

| Task | Information that must coexist | Main interaction | Continuity to preserve |
| --- | --- | --- | --- |
| Discover a useful reference trait | Reference, relevant source role, brief question and chosen facets | Select a trait, exclude a subject, add to comparison | The original source and scope of the preference |
| Compare territories | Actual candidates, meaningful differences, current question and shortlist | Pin a baseline, compare, annotate or keep open | Target versions, selected set and draft feedback |
| Express unclear dissatisfaction | Target, user's actual words and a relevant subset of diagnostic choices | Choose or reject a hypothesis; add words or view a contrast | Original statement, unresolved uncertainty and attribution |
| Refine a selected candidate | Source baseline, retained cue, proposed change and actual result | Revise plan, compare delta, continue or branch | Source lineage and the feedback being addressed |
| Evaluate an expression/use | Source, true variant and relevant size/surface context | Switch views, locate a flaw and compare an adaptation | Same subject and meaning across view changes |
| Resume work | Current question, latest usable results, unfinished input and unresolved decisions | Restore comparison or recover a failed activity | No repeated intake or manual search through chat history |

Use a dominant canvas/working region with subject selection and contextual detail. A left collection and right inspector can be one solution, not a required shell. Keep the decision's required information together. At narrow width use local layers, paired/toggle comparison and accessible return behavior; do not shrink all candidates into illegibility or fragment one judgment across unrelated routes.

Chat, canvas, reference browsing and history are complementary views over shared targets. The user may begin in any of them. Do not make the chat transcript the sole authoritative record or place every backend object in global navigation. A long document or large reference collection can scroll; preserve meaningful orientation without creating many competing scroll containers.

Workflow phase can recommend choices and views. It must not become a compulsory wizard that prevents revisiting references, comparing old versions or checking an application early. Separate “where the user is looking” from “what the design has decided.”

## Interaction contracts for a complete path

Before implementation, choose one end-to-end task and define its entry, target, available actions, effects, observed outcomes and return context. A useful first slice is:

`open an existing round → compare selected source versions → record partial preferences → agent forms the next-round brief → execute a declared activity → compare its results against the baseline → retain or correct the interpretation`

For each consequential action, identify the target/version, author, prerequisite, draft/commit semantics, operation owner, outcome and recovery. Keep the native service/component contracts with the future implementation. These are design obligations, not current API endpoints.

| Action or event | Required meaning | Failure or correction behavior |
| --- | --- | --- |
| Annotate a trait or region | The user's actual preference targets that object, version and facet/region | Keep the draft or failed record visible; do not report saved before confirmation |
| Record an agent interpretation | Attributed hypothesis derived from specified feedback | Correcting it preserves the original statement and affects future plans |
| Start an activity | Use the actual brief/plan/source/feedback input set within existing authority | Unsupported capability is explicit; repeated requests follow the real duplicate/replay contract |
| Change intent during execution | New opinion does not silently mutate the running request | Show which future work or amended plan can consume it |
| Receive a result | Bind it to its initiating inputs and actual status | Preserve partial results; late output does not replace current selection |
| Continue or branch | Retain the chosen cues and relevant inheritance scope | Branch-specific preferences are not automatically global |
| Resume after interruption | Reconstruct the durable question, targets, plan and results plus supported drafts | Mark missing or unsaved state honestly and keep legitimate recovery available |

Allow useful independent work while an operation runs. A whole-workspace spinner is usually inappropriate if comparison, annotation or research can continue. A blocked next action should explain its actual unmet prerequisite, not pretend all work is unavailable.

## Prevent common false implementations

| Superficial implementation | What is actually missing |
| --- | --- |
| Gallery with Like buttons | Target scope, trait-level preferences, next-round interpretation and recovery |
| Full tag catalogue beside every candidate | Phase relevance, manageable choices, free expression and intentional submission |
| Chat with attachment previews | Persistent shared targets, source/version identity, structured comparison and resumable decisions |
| Stepper for research/design/export | Real branching, revisiting and independent evidence/operation state |
| Generate button with a success toast | Input snapshot, partial/late output handling and connection to the design question |
| Editable cards for every record type | A task-based information architecture and complete path |
| Timeline containing all events | A useful explanation of the current state and the decisions that matter now |

Remove redundant module descriptions; keep guidance at the point where unfamiliar domain choices need it. Progressive disclosure is not a place to hide copy that serves no task. Do not use the method's documentation language as the product's navigation.

## Accept the experience through actual behavior

A design-only scope can finish with a coherent model, task flow and explicit assumptions. UI acceptance requires a running representative slice, accessible interaction checks and evidence of its promised behavior. A screenshot or field inventory alone is not sufficient.

Verify that a user can express mixed preferences without leaving the subject; the next output shows what it addressed; interpretation can be corrected; and browsing, waiting, refreshing or changing device layout does not erase supported state. Include a relevant late result, failed save, partial output and brief change when those capabilities exist. Keep no-image-reading instructions intact during implementation/review; use permitted behavioral and textual evidence and mark visual review unperformed when necessary.

Assess correctness of targeting, effort spent reconstructing context, ability to recover, and whether the next-round changes match the feedback. Do not use raw click count, tag count or visual density as a universal success score. If no user was observed, report a design/agent walkthrough rather than user validation.
