---
schema: document
status: active
description: Problem modeling, task state, project context and discoverability
links: [ ../AGENTS.md, ../documentation.md ]
---

# Project Context Guide

Use this guide when work is complex or ambiguous, when establishing a non-trivial project, or when changing its structure, workflow, or context. It applies to independent and delegated work. Use [placement rules](../documentation.md#artifact-placement-rules) to choose ownership and form, then consult the relevant local Module or [project Module](./project-context.md#keep-context-discoverable) for concrete resources and their writing conditions.

## Model before acting

- For complex or ambiguous work, reason from first principles before implementation. Identify the domain, boundaries, stakeholders, constraints, success criteria, assumptions, and trade-offs. Record the model, decisions, rejected alternatives, and unresolved uncertainties in the relevant task or design document using the applicable [expression forms](../documentation.md#structured-and-model-based-expression).
- Match the model's depth to the problem. Resolve a bounded ambiguity in the existing artifact; do not create a project-wide plan for a local fix.
- Understand the domain and design the solution through its [object model](data-state-modeling.md#object-models-as-the-design-basis): identify objects, responsibilities, relationships, attributes, operations and lifecycle before deriving structure or process steps. Reuse the existing model for unchanged behavior. Distinguish changing an object's definition from recording an execution transition or observation; update the appropriate owner.
- If problems recur, exceptions accumulate or sustained work leaves the cause unclear, pause affected execution and use [system diagnosis](systematic-troubleshooting.md). If the uncertainty is what the work contributes to the original outcome, first restore that relationship through [workflow change handling](workflow-management.md#respond-to-changes-at-their-owner). Resume with a causal explanation or discriminating hypothesis and a bounded plan; independent work can continue.
- For product capability proposals, roadmap decisions or a selected Delivery, use [feature planning](./feature-planning.md); keep accepted direction separate from executable task state.
- For technical design, follow the applicable [engineering design sections](./engineering-design.md#routing). Split complex features into cohesive parts with clear contracts and independent ownership potential; decomposition does not itself require delegation.

## Progressive derivation and externalized reasoning

When a design, data model, workflow, module boundary or state machine is complex or uncertain, start from the intended whole and governing model, then derive uncertain parts in visible reasoning steps. Keep each refinement connected to the promised result and shared invariants. These are reasoning steps within cohesive work, not separate Tasks or new acceptance targets. Explore alternatives with focused examples or small experiments and revise from the feedback. Use the scenario, dependency, interface or state view that exposes the actual uncertainty.

Use the nearest owning module or directory's `.sketches/` area for this working material when one exists, creating a focused note or experiment only when it reduces ambiguity. Keep several hypotheses or discarded paths distinguishable, and state when a sketch is exploratory rather than normative. Once a conclusion is stable, promote it to the owning `.docs/` design or specification, canonical source contract, or task record, and link the result back to the relevant sketch when the reasoning helps future changes.

This process is for meaningful complexity, not a ritual for every edit. A simple bounded change can stay in the existing source or task context; use a sketch when externalizing the reasoning makes gaps, trade-offs, feedback, or the next derivation step easier to inspect.

## Make method execution explicit

When applying a non-trivial design, review, planning or implementation method, make its working progress discoverable in the existing task, working document or linked artifact. Select the useful steps/questions and how to record their results before proceeding. The purpose is operational memory: know what has actually been examined, what conclusion can be reused and what remains unresolved.

| Point in the work | Prompt for the Agent's working record |
| --- | --- |
| Enter a method | What scope/questions apply, what is already established, and what is the next useful action? |
| Reach a meaningful result or decision | What was examined or produced, what conclusion follows, and what source/model supports it? Retain relevant uncertainty or excluded cases |
| Change assumptions or resume work | Which prior conclusions remain valid, which are affected, and what must be revisited? |
| Finish or hand off | Which questions are resolved, including sound areas, and which are incomplete, blocked, deferred or inapplicable with a reason? |

Use a few notes, an annotated model, a small map or linked source/check result as appropriate. These prompts are not required headings, fields or a document template. Record at meaningful transitions rather than after every command; do not copy tool transcripts, canonical declarations or the same status into several documents. A bounded operation may need only one conclusion in the existing record.

Agent confirmation means the producing owner checks an intermediate result against relevant sources and constraints. Consumers use established results under [verification ownership](workflow-management.md#bound-verification-and-integration); each handoff does not require another independent factual check. It does not add a user-approval step; consult the user only for an actual unresolved decision or authority boundary. Record observations as work proceeds rather than reconstructing a ceremonial checklist at the end. Reading a guide or marking a box does not establish that its method was applied.

Keep overall Task execution status in its canonical metadata. Maintain domain working state and transitions using the professional owner's existing model or a small current-facts/next-evidence representation, as [Task state maintenance](task-records.md#maintain-task-and-domain-state) describes. This is distinct from the Task lifecycle and the product/runtime object's state; it is not a prohibition on domain state diagrams or an instruction to create a duplicate status machine. Link durable design conclusions from the task; on changed inputs, revisit affected conclusions without repeating unrelated completed work.

## Maintain project and task state

- Establish a project control space before or alongside non-trivial implementation. Before substantial work on a complex task, create or update its task record and externalize the working model. A fresh agent must be able to discover current state, constraints, authoritative sources, and next actions.
- Keep design rationale in the design document and execution state in the task record; link between them. Record the outcome, scope, owner, status, dependencies, blockers, next action, and completion evidence at meaningful changes.
- Update an existing relevant record before creating a new one. Ordinary small fixes do not require new design or task documents. Use [task record guidance](./task-records.md) when a durable record is useful, adapting its content to independent or delegated work without a fixed form.
- Maintain professional method/profile entries separately from live assignments. For coordinated work, recover the [workflow definition, original work-item state, derived readiness and actual Agent bindings](workflow-management.md#keep-the-definition-and-execution-facts-distinct); do not merge them into a second status ledger.
- Keep the current summary organized by intended capability, established result, structural or integration gap, and next coherent action. After handoff or compaction restore that summary, the governing model and the Task's contribution before resuming a command queue. Link detailed run history; do not make a fresh Agent infer the current system from appended logs.
- For delegation, consult the [delegation criteria](../roles/controller.md#when-to-delegate) and the applicable role guide. Project context management does not require a controller or extra agents for independent work.

## Keep context discoverable

- Use the relevant `AGENTS.md` as the context entry point. Task context, constraints, decisions, plans, and deeper references must be reachable through its links; they need not all appear in that file.
- Let directory and filename choices communicate purpose. Use [structural placeholders](../documentation.md#structural-placeholders) where an empty location or name helps future work; they carry no current assignments or prescribed document sections.
- Update entry points and context when project structure, workflow, key decisions, or structural design changes. Follow the [context authoring rules](../documentation.md#agent-entry-points-and-context).
- A feature is complete only when a fresh agent can discover, understand, use, and modify it. Add module-level `AGENTS.md` files, documentation links, and entry-point comments where needed to explain responsibilities, canonical contracts, and intended use.

## Check a context path through actual work

When structure or context routing changes, start from a real request and verify three paths: discover the responsible public contract/current material; modify through the authorized owner and its focused checks; recover the Task, current basis and next action after handoff. A README route must make a required maintenance entry reachable; a file's existence or possible automatic loading is not proof the reader encountered it.

Use the directory tree for primary maintenance ownership and explicit links for uses, implements, composes, derives-from, supports or supersedes relationships. These are relationship meanings, not new mandatory metadata. A directory, a package and an Agent assignment need not share a boundary. Native exports enforce only their actual package resolution rules; names such as `internal` are not filesystem isolation.

Keep public usage and effect/recovery semantics reachable for consumers. Load internal maintenance, generation, migration and release instructions only for those actions. Preserve distinctions among current contracts, proposals, generated projections, fixed evidence and task state. Do not add the same six folders or a Controller to every scope; split only when a real responsibility becomes easier to find and maintain.
