---
schema: scenario
description: Turn product needs and review findings into approved roadmaps and outcome-based deliveries
tags: [product, planning, delivery]
---

# Feature Planning and Delivery

Use this guide when reviewing product direction, proposing capabilities, maintaining a roadmap or turning a selected outcome into an implementation plan. Small, already-authorized changes can go directly to a [Task](./task-records.md). Planning does not itself authorize implementation or delegation.

## Planning levels

When an evolution proposal needs users to see the affected current/target scenarios, modules or interfaces together, compose [design management](../domains/design-management/AGENTS.md). Keep planning choices, design decisions and execution state in their Roadmap/Proposal/Delivery/Task owners below; the connected views reference them rather than becoming another plan.

| Level | Owns | Boundary |
| --- | --- | --- |
| Roadmap | An owner's capability direction, selection, relative priorities and meaningful horizons | Broader than one Delivery; capability selection does not report implementation progress |
| Proposal | One change's reason, affected current models, intended differences, design decisions and unresolved questions | A separately addressable design change for planned capability work; acceptance does not establish implementation |
| Delivery | A bounded, verifiable outcome and the plan to deliver it | Implements an explicit portion of one or more Proposals; size follows the outcome |
| Task | Executable work with an owner, scope, dependencies and acceptance boundary | May be independent or contribute to a Delivery; use subtasks only when useful |
| Current model | Domain meaning, selected targets and navigation to topic/module designs, native contracts and technology decisions | A distributed design entry, separate from execution records; show actual support and remaining targets |
| Controller | Coordination of its assigned outcome and the associated task/dependency graph | May own one Delivery or feature stage; need not own the whole product roadmap |

```text
Roadmap capability item -> Proposal -> Delivery -> Task
current model -> proposed change -> detailed design and native contracts
established Task results -> current model and support update -> Delivery acceptance
```

Use Roadmap → Proposal → Delivery → Task as the default decomposition for planned capability changes. A Proposal may originate before roadmap selection; an unselected direction needs no downstream artifacts. A small fix with a clear model and contract can go directly to a Task without creating the parent documents. A direct user instruction can supply the decision and execution authority together. Reuse that authority; do not ask again for ordinary work within the agreed scope.

Use the native [Roadmap](../refs/xschema/schema-contract.md#roadmap), [Proposal](../refs/xschema/schema-contract.md#proposal), [Delivery](../refs/xschema/schema-contract.md#delivery) and [Task](../refs/xschema/schema-contract.md#task) contracts for their metadata. Follow [placement rules](../documentation.md#artifact-placement-rules) for new files: create Delivery records as `.controller/<outcome>/<outcome>.delivery.md` and their Tasks as `.controller/<outcome>/tasks/<responsibility>.task.md` under the responsible owner, retaining the matching content schemas. Retain valid existing homes and expose their roles through the owner's real artifact catalog; do not relocate history or duplicate a model to match the defaults.

## Keep relationships and states with their owners

The upstream object owns formal aggregation: Roadmap items list Proposals, Proposals list Deliveries, Delivery stages list Tasks, and Tasks retain their existing child decomposition. A capability item is local to its Roadmap, like a stage key is local to its Delivery; neither requires its own entity file or global ID. A Proposal can affect several capability items, and several Proposals can share a Delivery when its covered portions are explicit. Keep aggregation, task decomposition, management and execution dependencies distinct, under their native contracts.

Lower-level prose links its source change for recovery. Derive reverse views from the upstream relationships and preserve query coverage limits; do not maintain mirrored parent fields or treat an incomplete scan as proof of no relationship. The Proposal's model references locate affected design sources; general background stays in ordinary links. Reference actual readable files or sections, and describe planned destinations in prose until they exist.

Keep each state with the subject whose decision it describes. These are recommended vocabulary and transition meanings, not new closed schema enums:

| Subject | State meaning | Update when |
| --- | --- | --- |
| Roadmap | `draft`, `active`, `paused`, `retired` describe the overall planning baseline | Its owner adopts, pauses, resumes or replaces that plan |
| Capability item | `candidate`, `selected`, `deferred`, `dropped` describe planning choice | User direction, value or dependencies change; an absent state remains undeclared |
| Proposal | `draft`, `proposed`, `accepted`, `rejected`, `withdrawn`, `superseded` describe the design decision | The change is prepared, selected, declined, withdrawn or replaced; link a successor when it supersedes an accepted baseline |
| Delivery or Task | `planned`, `active`, `blocked`, `completed`, `cancelled` describe execution within that object's scope | Its own start, blocker, acceptance or cancellation condition holds; a local blocker or completion does not propagate automatically |

A selected capability may already be delivered and remain a planning priority. An accepted Proposal stays a design decision when its Deliveries complete; do not turn it into `completed` from Task counts. Preserve actual support separately in the current model by consumer, environment or version. Runtime domain state, such as a running ReportRun, belongs to its domain contract.

Keep a Proposal's internal design decisions in named body sections or a short table, linking long details to their topic owners. Identify which parts are `accepted`, which remain `open`, and which are `deferred`, `rejected` or `superseded`. Proposal acceptance must identify its usable baseline; an open question that blocks this batch needs an owned design decision before dependent implementation starts. Do not create a second structured change registry.

For example, a Proposal may accept original Run identity and independent observer lifetime while leaving UI navigation open. A CLI Delivery can complete after its scenario and required model updates hold; the Proposal remains accepted, and the current model says CLI is supported and UI is still a target. Completing every listed Delivery does not prove that they cover every Proposal target. Cancelling a later UI Delivery preserves the delivered CLI facts and does not itself reject the original design.

## Carry a model change through the work

Treat each proposed capability as a change or refinement of the relevant model: domain meaning, a scenario promise, composition, interaction or an engineering constraint. A feature need not introduce an entity. Semantics-preserving work states the guarantee it retains. One capability may evolve through successive Proposals, and one Proposal may need several Deliveries. Reuse one Proposal when the same design change affects several capabilities; do not duplicate its decisions.

| Work record | Preserve in the existing record or direct links |
| --- | --- |
| Roadmap item | Intended capability, value, entry conditions, relative order/horizon and selected Proposals; follow them to Deliveries instead of duplicating their aggregation or execution states |
| Proposal | Relevant current scenario/model, intended change or refinement, retained guarantees, affected contracts/consumers, choices, deciding examples and aggregated Deliveries |
| Detailed design and native contract | The proposal change being refined, concrete rules and boundaries, unresolved design questions and current versus target scope |
| Delivery | A complete outcome implementing an explicit portion of the selected change, excluded/remainder scope, responsible Tasks and model-update ownership |
| Task | Its parent outcome/change when present, owned design or behavior, canonical inputs, exact update destinations and completion boundary |
| Current model | Established semantics and actual support; keep accepted but undelivered targets distinguishable and retain the original decision's rationale by reference |

A few sentences and heading links can carry these relationships. Keep exact fields, signatures and command grammar in native declarations; do not introduce a change registry or repeat a proposal's rules in every Task. Product/design owners resolve what changes, domain owners refine their contracts, and the Controller ensures the selected outcome and necessary model updates have owners.

## Derive proposals from the product

Start with the product's intended users, core jobs, differentiating capabilities and accepted design principles. Review actual gaps against that baseline; a technically interesting feature or an isolated review finding is not automatically a product priority.

When the input is an existing product review, use [path review and finding disposition](../domains/product-design/evaluation.md#review-an-existing-product-path) to separate unmet chosen guarantees, candidate new promises, unresolved intent, evidence gaps and outdated explanations. Bring actual product changes into proposal selection; route bounded corrections or verification to their existing owners. A source-based risk is not automatically an observed user failure.

Locate the affected scenario and current domain model before comparing proposals. Explain which concept, responsibility, rule or observable promise would change, and follow its canonical contracts to the affected consumers. Compare alternatives on those differences. Reuse unchanged semantics; where the model is missing, reconstruct only what the decision needs through [collaborative modeling](collaborative-modeling.md). Current implementation supplies observations, not the design rationale by itself.

| Review lens | Establish | Deeper guidance |
| --- | --- | --- |
| Product capability and experience | Which user can accomplish which job, where the journey fails, and whether the proposed capability fits the product | Relevant product requirements; [experience review](../domains/webdevelopment/review.md#experience-review) for web products |
| Product structure | Whether capabilities, domain responsibilities and composition support the intended workflows | [Product composition](./engineering-design.md#domain-module-and-product-composition) |
| Domain design and implementation | Concrete constraints or defects in frontend, backend, protocol or other affected domains; shared contracts and compatibility to preserve | The owning domain guide; [architecture review](./architecture-quality-review.md) for engineering boundaries |

Use only the relevant lenses. Review can generate new proposals, while explicit user needs can already provide sufficient evidence; neither a full-product review nor a separate review team is a prerequisite for each change.

Keep early ideas in the owner's sketches or existing review. For a planned capability change, create a separately addressable Proposal using the [default home](../documentation.md#artifact-placement-rules), or maintain an existing valid Proposal home. Link detailed designs instead of copying them:

`problem + current model -> intended outcome + product fit -> changed and retained promises -> scope/dependencies -> alternatives and open decisions`

Name the affected model sections and explain what is added, refined, composed, changed or removed, including existing consumers, data and in-flight work where relevant. A proposed button or endpoint alone does not explain the change. Leave detail to its owning design when it is not needed for the product decision; make that unresolved design work visible before implementation is assigned.

Prefer changes that complete core journeys, resolve demonstrated problems or enable accepted capabilities. Compare user value, urgency, dependency impact, uncertainty and implementation/maintenance cost. Preserve coherent domain and design principles; explain any proposed change to them. Do not prioritize by novelty, implementation convenience or the number of findings.

## Confirm direction and maintain the roadmap

Discuss unresolved product intent, scope and trade-offs with the user. Present a concrete recommendation and meaningful alternatives. Mark a capability selected when the user's decision supports it, and record Proposal acceptance separately for the chosen design baseline. Reuse decisions already supplied in the assignment. Unconfirmed ideas stay visibly separate as candidates, with links to their evidence and Proposals when available.

A roadmap entry needs only the useful parts of:

`outcome; product value; priority/horizon; prerequisites/uncertainty; Proposal reference; planning choice`

Record the accepted scope and exclusions. Acceptance of a direction does not automatically authorize immediate execution of every item, and choosing one item does not accept adjacent proposals. Existing authorization remains sufficient; do not turn routine implementation choices into new product approvals.

Maintain relative ordering where useful; promise dates only with a credible basis. Revisit affected entries when user priorities or material evidence change, rather than rewriting the plan after every task update. Link execution records; do not duplicate their live statuses in the roadmap.

Keep Proposal decision state with its original decision. A Roadmap records direction and follows its Proposals to delivered scope; it must not imply that an accepted Proposal is already delivered. Deferred or cancelled work retains its rationale, established effects and the current model that still applies.

## Turn an outcome into a Delivery

When the user selects an outcome to advance, use the smallest useful plan:

| Situation | Plan |
| --- | --- |
| Bounded change with clear scope and acceptance | Execute directly; use a Task when durable state or handoff is useful |
| Outcome spanning domains, dependent work or meaningful acceptance milestones | Create or update a Delivery linking the required Tasks |
| Large outcome with independently valuable and accepted sub-outcomes | Link smaller Deliveries from the owning plan and state how their outcomes satisfy the parent |

A smaller Delivery is useful because it has its own acceptance boundary, not merely because another level can be created. Express parent/child Delivery relationships and cross-plan prerequisites through explicit plan links and outcome conditions; the current schema has no dedicated nested-Delivery relation. Keep `stages[].tasks` for Task paths, not Delivery paths. Do not copy child task inventories or infer management authority from these links.

For the selected outcome, establish:

1. **Capability and acceptance:** the actor, scenario, expected behavior, selected proposal/model change, the portion delivered now, exclusions and observable evidence of success.
2. **Domain contracts:** responsibilities, owned facts, public inputs/outputs, failure behavior and composition points. Follow [contract-first design](./engineering-design.md#types-and-contracts-before-implementation); resolve the contracts needed for the next work without specifying every private implementation.
3. **Executable work and dependencies:** cohesive domain Tasks, including unresolved design and native-contract refinement, their scope and the actual conditions to start or finish. Identify which goals/constraints are selected promises and which are provisional hypotheses. Where the domain question is open, assign the question and its consumers; let the qualified owner derive professional objectives, alternatives and useful decision evidence instead of fixing the answer in the Task. Independent implementations can proceed once their contracts are usable; integration still requires real cooperating capabilities.
4. **Delivery closure:** an owner for composition/integration, verification of the actual user outcome and required model/contract updates at named destinations, plus applicable migration and resource work. Task completion or mock-only evidence does not establish the whole Delivery.

Before coordinated dispatch, use [workflow management](workflow-management.md) to model professional responsibilities, dependencies, decisions and structural result contracts. Each result specifies its function, consumer, meaning, invariants and use conditions before its document or implementation location. Domain owners make professional design decisions; the Controller coordinates those owners and the overall outcome.

Derive stages and Tasks from the selected model change, complete outcomes and responsibility boundaries. Explain how each required result contributes to the Delivery and which responsibility consumes it, using [result definition](workflow-management.md#define-what-each-result-must-accomplish). Check that the planned contributions together cover the promise and its shared invariants; neither a Task list nor links alone establish coverage. Link the original model/contracts and assign their required updates. A semantics-preserving fix can reuse this basis and proceed directly.

Assign design and model maintenance with the owner implementing that responsibility where possible. Use a separate design Task when a consequential unresolved decision blocks several consumers; its usable decision/contract releases those dependents. If another owner must merge a shared model, name that receiving Task and its completion dependency. Do not leave model maintenance as an unassigned post-release intention.

Assign a bounded Delivery to a domain Controller when it needs its own professional workflow; a Task can also carry a finite coordination responsibility. Role follows the work and actual authority, not the record type. Nested coordination does not add a new formal planning relation or transfer professional decisions to the parent Controller.

Use few, outcome-based stages and sustained domain Tasks under the [whole-domain principle](../principles.md#whole-domain-authoring). Do not mechanically multiply pilot/full stages by native/API/client/test layers. A feasibility pilot resolves a named risk; its full acceptance is a prerequisite for further implementation only where it establishes a fact that implementation actually needs. Revise inherited Task boundaries that fragment a complete capability.

Keep design, the complete declaration/module batch, consumer binding and ordinary repairs together. Let usable contracts release other owners while the domain owner continues its complete assignment. Plan concentrated system verification separately from implementation division; no per-Task environment, packaging or acceptance cycle is implied. Cases, parameter combinations, capture batches and per-file changes are execution details. Separate responsibility only for actual professional boundaries, useful substantial parallelism or an independently meaningful outcome; record real start conditions and final acceptance independently.

Keep formal dependencies with their canonical [stage](../refs/xschema/schema-contract.md#delivery) or [Task](../refs/xschema/schema-contract.md#task) owner; the ready queue is derived from those conditions and current evidence. For authorized parallel work, use [Controller scheduling](../roles/controller.md#schedule-ready-work), then the existing task/handoff guides.

## Close the loop

Accept a Delivery against its promised outcome and the evidence actually obtained, including whether the contributions compose under the shared domain model. Apply [outcome acceptance](workflow-management.md#recover-and-close-the-outcome) in the existing integration work. Resolve material responsibility, contract or consumer gaps even if individual Tasks passed. Keep the Delivery open while its own conditions remain unmet, with remaining work and owners clear; preserve the Roadmap → Proposal → Delivery reading path without mirroring states upstream.

Check that the implemented portion is incorporated into its current model and contracts at the claimed support level. A design-only Delivery may finish with a selected target and usable contract, without claiming implementation. For staged implementation, merge only the established portion and leave the remaining target with its proposal and later Delivery. A feature spanning several outcomes is not fully delivered merely because one Delivery closed.

An accepted plan does not make its diagnosis or technical recommendations infallible. Professional owners correct compatible assumptions and methods with evidence in the canonical Task/model. When detailed design or implementation changes an actual accepted promise, return the concrete difference and affected consumers to the proposal/decision owner and reconcile dependent designs and Tasks. Compatible refinements within existing authority proceed directly. Concurrent Tasks modifying one rule use a named owner and bounded write coordination; recheck the latest rule before merging rather than overwriting another change from an old baseline.

Classify implementation findings through [workflow change handling](workflow-management.md#respond-to-changes-at-their-owner). New opportunities return to proposals when they change product scope. Compatible work needed for the authorized outcome stays within that work; it does not repeat the roadmap decision cycle. Missing evidence or an intentional out-of-scope rejection does not automatically create another capability commitment.
