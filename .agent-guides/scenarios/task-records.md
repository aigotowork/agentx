---
schema: document
status: active
description: Proportionate task records for independent and delegated work
links: [ ../AGENTS.md, ./task-handoff.md ]
---

# Task Records

Use a task record when the work needs durable state or an assignment handoff. Choose its owner using [placement rules](../documentation.md#artifact-placement-rules), then locate the concrete task area in the relevant local Module or [project Module](./project-context.md#keep-context-discoverable). Continue the relevant existing record. For a new Task, create `<responsibility>.task.md` under the owning `.controller/`, for example `.controller/release/tasks/update-guidance.task.md`; use the [placement rules](../documentation.md#artifact-placement-rules) for independent work and legacy records. The `.task.md` suffix is required for discovery by the supplied xschema defaults; it does not replace `schema: task` or required metadata. Use the [xschema Task structure](../refs/xschema/common-schema.md#task) in YAML front matter. Choose body headings or compact notes to fit the assignment; no body template is required.

A Task owns executable work within the default Roadmap → Proposal → Delivery → Task decomposition: the Roadmap selects capabilities, the Proposal owns the design change, and the Delivery owns a bounded outcome. Use [feature planning](./feature-planning.md#planning-levels) when choosing the level. A small fix can link its existing model and contract without creating parent documents; use the owner's actual task area or the [new-work defaults](../documentation.md#artifact-placement-rules).

## Useful information

Treat the Task as a [domain object](data-state-modeling.md#object-models-as-the-design-basis): its definition establishes the work boundary, inputs and acceptance rules; its lifecycle records what actually happens to that work. For starting, blocking, resuming, completing or cancelling work, establish the actor's authority, applicable prerequisites and resulting facts before updating the canonical status. Apply the existing lifecycle and metadata contract; no extra runtime object or status registry is required. A dispatch request or worker observation alone does not establish a Task transition.

Explain the result's contribution to the original outcome, governing model, consumer and completion boundary using [result definition](workflow-management.md#define-what-each-result-must-accomplish). Name actual inputs and update locations after establishing that responsibility; paths or parent links alone are not acceptance criteria. For a delegated Task, bind the actual assignment and link the professional entry, role, decision authority and any delegation limit. The assigned Agent owns keeping the Task current as well as producing the domain result. The record type does not prescribe role.

Make the intended outcome and current state easy to find. For delegated work, make the actual owner, scope, authority, dependencies, and completion boundary clear enough that the worker can act without guessing. Add checks, resources, cleanup, or follow-up links only where they matter to the assignment.

Apply [context, not control](../principles.md#context-not-control). Distinguish real user/authority commitments and current facts from Controller hypotheses, suggested methods and open professional questions. Link the source of a material constraint. The expert may refine the professional problem, objectives and evidence from domain reasoning; do not turn an unverified Controller preference into a technical requirement or acceptance gate. Record a correction and its basis, coordinate affected shared promises, and continue compatible authorized work.

Link a designated executor's check result once where it supports a decision. Do not copy its file inventory, hashes or run history through worker, parent and architecture records, or refresh unchanged records for each received report. Create a separate receipt only when an actual consumer or publication contract requires it.

For a design or implementation change, identify the selected Proposal/Delivery portion when present, affected scenario and original rule/contract, semantic change or retained guarantee, and any design question still owned here. Assign current-model/native-contract updates or an exact handoff to a named receiver. Keep this concise context in body links; upstream records own formal aggregation. Do not add reverse parent fields or a model registry. A bounded correction reuses its established model and contribution; it needs no new design artifact.

For a task that applies a substantive method, retain its [working progress](./project-context.md#make-method-execution-explicit): selected questions, actual intermediate results, resolved conclusions and remaining uncertainty. The task may link an existing review/model instead of copying it. Adapt the record to the work; no fixed checklist, per-command log or additional approval gate is required.

Name the actual context or input needed to start and the meaningful completion/decision boundary. For discovery, unresolved domain signatures and methods are expected outputs, not prerequisites to invent in advance. The professional owner refines proposed acceptance criteria against the actual goal and evidence; a desired answer is not a valid criterion merely because it appears in the Task. Store formal edges at their canonical [decomposition](../refs/xschema/schema-contract.md#task) or [stage](../refs/xschema/schema-contract.md#delivery) owner; link them rather than adding reverse dependency fields. Keep task progress distinct from [derived readiness and worker availability](../roles/controller.md#schedule-ready-work). When reassigning a worker, preserve the old Task's evidence and establish the new binding and scope explicitly.

## Maintain Task and domain state

The assigned Agent maintains this record throughout the assignment; completing source edits while leaving a stale Task is an incomplete handoff. Update at meaningful changes to understanding, decisions, evidence, blockers or transitions, not after every command. A later Agent must recover current facts and the next condition without reconstructing a log.

| State being described | Owner and representation |
| --- | --- |
| Task execution lifecycle | Canonical Task metadata under the existing native contract; `blocked` requires `blocking_reason`, removed on resumption |
| Domain work's current progress or maturity | The professional owner uses the domain's existing workflow or the smallest appropriate transition/state/fact model in the Task or a canonical linked artifact |
| Product/runtime object's state | Its actual domain declaration/store; the Task references relevant observations, never becomes a second authority for that object |

For substantive stateful work, make the current state or independent dimensions, relevant guards/transitions, supporting facts and next required evidence explicit. Reuse a transition table, state diagram, event flow or maturity model from the domain; use a short current-fact/next-condition note for a bounded change. Do not prescribe a universal sequence, new metadata enum, library, percentage or diagram for every Task. A task can remain active while a design is selected, one integration is blocked and another is ready. Preserve those distinctions without duplicating the Task lifecycle or changing a product object's state by editing Markdown.

Keep design and canonical contracts with their actual owners. Link the professional model and update the current view when its conclusions change. Replace stale blockers and next actions, retain useful superseded reasoning as clearly historical evidence and avoid a second editable status ledger. The Controller consumes this record rather than requiring periodic duplicate reports. See [domain state modeling](data-state-modeling.md#when-to-use-a-state-machine) for state semantics, not another compulsory workflow.

## Completion and handoff

State which contribution is now usable by the original or parent outcome, how it fits the governing model and consumer, and what remains to integrate. Link relevant checks and their limits, model/contract updates and cleanup. Use the [scoped and composition judgments](workflow-management.md#recover-and-close-the-outcome); local test success alone cannot establish that contribution. Independent work uses the original requested outcome without creating a parent record.

Complete only the declared boundary. An explicitly assigned handoff may finish before integration when its receiver and remaining responsibility are clear in the original Delivery. Task closure does not complete that Delivery or change Proposal/Roadmap state. Remove obsolete blockers/next actions while preserving useful evidence by reference. Use [task handoff](./task-handoff.md) for authorized dispatch.

Return established semantic changes to the owning current model and native contracts, or complete the Task's explicitly assigned handoff to a named merge owner. Keep the reason linked to the original proposal/decision; do not leave the only current definition in a completed task's appendix. Report which portion became current and which accepted target remains unimplemented; a handoff is not a completed merge. If semantics are unchanged, retain the original model and report the relevant check rather than creating a new design artifact.
