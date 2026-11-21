---
schema: document
status: active
description: Object models as the basis for domain data, structure, behavior, lifecycles and workflows
links: [ ../AGENTS.md, ./engineering-design.md, ./project-context.md, ../documentation.md ]
---

# Data and State Modeling

Use the object model to explain what the domain is and how it works before choosing a solution. Identify objects, responsibilities and relationships, then derive data, structure, behavior and workflows. The data model defines meaningful facts and valid combinations; domain operations define allowed behavior; the lifecycle model defines how state and facts may change; workflows compose those operations. Use [object models as the design basis](#object-models-as-the-design-basis), then [data and types](#data-and-types) or [state transitions](#state-transitions) for the affected question. This applies to software resources, work items and design records. Reuse a sound existing model; simple records do not require a state machine.

For frontend component design, use the [frontend modeling bridge](../domains/webdevelopment/interaction-and-state.md#model-behavior-before-presentation) to apply these definitions to data ingress, component-owned behavior and state-to-view projection.

For simple content, express it directly. For new models, complex relationships, interacting states or multiple dimensions, apply [visual model communication](interactive-design-communication.md): derive a polished panoramic HTML view from the domain model, compose professional visual forms such as Mermaid, SVG, Canvas, chart libraries or 3D renderers, deliver the portable componentized HTML file early; when shared access is needed, serve that same file through LAN HTTP and update it with the canonical model. This also applies to systems without a frontend.

## Object models as the design basis

Start from the intended outcome and concrete scenarios. Explain which domain objects exist, what each is responsible for, how they relate and which rules govern their collaboration. Use that explanation to assess a proposed solution and locate behavior at its owner before selecting implementation mechanisms or listing process steps. Express the model through domain concepts, relationships and operation contracts, using classes or records/functions as appropriate.

Before introducing an object or changing its responsibilities or behavior, establish the following in its existing model or native contract. Reuse established answers and deepen only the affected part; these are modeling questions, not required fields or separate documents.

| Concern | Establish |
| --- | --- |
| Meaning and ownership | What the resource represents, its identity where applicable, its responsibility boundary and the owner of its rules |
| Attributes and relationships | Authoritative facts, related objects, valid combinations and the consistency boundary protecting them |
| Lifecycle | State meanings, state-specific facts and invariants, creation and end conditions, and allowed transitions where behavior depends on state |
| Operations | Meaningful queries and commands; inputs, actor/authority, guards, results, changed facts, effects and rejection or recovery behavior as applicable |
| Workflow use | Who calls each operation, under which conditions, what result or event permits the next action, and who handles waiting or failure |

For a command that changes state, connect `resource + command + context` to either an accepted transition with resulting facts and effects, or an explicit rejection/unchanged result. Queries expose facts without implying a lifecycle advance. An operation may be an object method, a function or a service contract; its owner must enforce the same rules. Follow [domain objects and behavior](#domain-objects-and-behavior) for implementation choices.

Derive process steps from these resource operations. Keep each resource's transition rules with its owner, and define the interactions between separate lifecycles. A workflow may have its own state, but it must not bypass guards by directly rewriting another resource's status or maintain a competing copy of that status. When several resources must change together, identify the real consistency boundary or coordination/recovery contract.

Apply this reasoning while writing and maintaining records as well as code. Creating a Task establishes a work resource; starting, blocking, resuming or completing it changes that resource under its existing rules. Creating an assignment, observing an Agent and accepting a Delivery concern related resources with their own facts. Use [Task records](task-records.md) and [workflow management](workflow-management.md#keep-the-definition-and-execution-facts-distinct) for their authoritative application. A design record's acceptance state is separate from the lifecycle of the resource it describes.

Keep the model and actual state coherent as work proceeds. A new lifecycle case or changed operation updates the owning contract and affected callers, representations and checks; a routine execution records the established transition and evidence in the resource's canonical record. Immutable values need valid construction and value constraints, and stateless capabilities need operation contracts; neither needs an invented lifecycle.

## When to use a state machine

Use an explicit state machine when the legal next behavior depends on the current state or when timing, retries, cancellation, concurrency, or late results can change what an input means. Typical signals are a lifecycle with named stages, commands that are accepted in some states and rejected in others, a protocol or resource with legal call order, asynchronous work with result races, or several flags whose combinations are difficult to explain. Use a statechart when states share behavior, contain substates, or evolve in parallel.

Keep a plain record, a typed function, or a small guarded operation when behavior is stateless, transitions are unconditional, or the lifecycle is short and obvious. Do not introduce a state-machine framework merely because a value has a `status` field. A state enum, diagram, or library does not enforce runtime transitions by itself.

```text
Use FSM when: meaning(input, state) or legal(nextState) varies by state.
Stay simple when: nextState = f(input) has no lifecycle-specific guards or effects.
```

## Data and types

Start from the affected scenario and existing domain language. For a new or changed concept, explain what it means, how a concrete example differs from a nearby concept, who owns its facts and which rules depend on that distinction. Identify relationship cardinality and optionality where they constrain behavior. A field list, database table or shared name is insufficient to establish domain meaning. Use [collaborative derivation](collaborative-modeling.md#derive-concepts-and-behavior) when examples expose an unresolved boundary.

- Identify entities by stable identity and value objects by their values. Give identifiers, units, and constrained values distinct types where confusion is plausible; prefer immutable values. Use the language's natural types rather than requiring wrapper classes for every primitive.
- Define relationships, ownership, and invariants before storage layout or concrete methods. Keep facts that must remain consistent within an explicit consistency boundary; route changes through its owning domain operations. Use an aggregate when that boundary warrants it, and define cross-boundary coordination only when required.
- Make invalid combinations difficult to represent. Use variants or discriminated unions to associate each lifecycle state with its required data, instead of unrelated booleans and optional fields. Keep genuinely independent dimensions separate and derive redundant flags from canonical state.
- Types constrain representable data; they do not by themselves enforce legal transitions, runtime input, or concurrent updates. Check external data at its boundary and enforce remaining invariants in the operations that own changes.

Maintain meanings and relationships in the owning domain topic or nearby declaration comments, and exact types in the native declarations used by code. Link them from the existing model entry. Mark current rules, selected targets and unresolved alternatives clearly; do not use a runtime object's `status` to report whether its design is accepted or its implementation is supported.

### Domain objects and behavior

When a domain has a clear entity or value object whose data, behavior and invariants belong together, prefer an object-oriented model if it makes those responsibilities clearer. Use a class to express that model when it fits the language and existing design. Start from domain meaning and operations, not a database table or a collection of getters and setters.

Keep valid construction, identity or value equality, and state changes under the object's control. Expose meaningful operations such as `order.cancel()` that enforce rules; unrestricted setters allow callers to bypass that responsibility. An aggregate root may own changes that must preserve invariants across related objects. Keep persistence, transport and application orchestration outside these objects, using [capability contracts](./engineering-design.md#service-contracts-and-dependency-inversion) where collaboration is needed.

Choose composition over inheritance when it keeps responsibilities independent. Immutable records with functions are suitable when they express the same rules more directly; data transfer records and pure transformations need no class. A service coordinates or supplies a capability, while an entity owns identity and domain behavior; either may use classes, but they need not share the same implementation shape.

#### Encapsulation, inheritance and composition

Apply object-oriented principles to the model's responsibilities and observable behavior. Choose the language mechanism after establishing those promises; a class hierarchy or a named pattern is not a modeling outcome by itself.

| Design decision | Method and deciding question |
| --- | --- |
| Cohesion and encapsulation | Keep facts and operations together when they protect the same rules and change for the same domain reason. Identify every mutation path, including exposed collections and borrowed references: can a caller bypass an invariant without invoking its owner? Separate transport, persistence and orchestration when they have independent responsibilities. |
| Valid construction | Establish required facts before making an object usable. Use constructors, factories or validated functions according to acquisition and failure needs. Keep asynchronous resource acquisition and partial cleanup explicit. Reconstruct persisted or transported facts through an owned path that checks the relevant invariants; a type assertion does not restore an instance or validate its state. |
| Inheritance and substitutability | Use inheritance for a genuine behavioral subtype whose instances can satisfy the base consumer's promises. Compare accepted inputs, preconditions, results, failures, effects and lifecycle: the subtype must not demand stronger preconditions or weaken promised outcomes. Replay a base caller with the subtype. Shared fields or a desire to reuse code alone do not establish this relationship. |
| Composition and delegation | Compose objects when responsibilities or variation can change independently. Define the collaborator's capability, who creates and owns it, whether it is shared or borrowed, and how failure and cleanup propagate. Delegate through that capability while retaining each owner's invariants; moving code into a helper does not itself establish a responsibility boundary. |
| Interfaces and abstract classes | Use a capability interface when callers need a substitutable promise across implementations. Use an abstract class when a real family shares cohesive state or behavior under a stable extension contract; make overridable behavior and construction order safe. Keep a concrete class or function when no such boundary is needed. |
| Extension and dependency direction | Introduce a strategy or other variation point for an actual independent policy or mechanism. Keep required ports with their consumer and wire implementations at composition. Assess a realistic change: can it remain with its owner without altering unrelated consumers or bypassing domain rules? |

Use these questions for entities, stateful Services and resource owners as applicable. Connect collaborator boundaries to [Service contracts and dependency inversion](engineering-design.md#service-contracts-and-dependency-inversion), and keep [representation mappings](#domain-storage-and-transport-representations) explicit. TypeScript structural assignability and access modifiers help express a design; they do not prove behavioral substitutability, runtime validation or safe mutation of nested references.

### Domain, storage and transport representations

When an entity crosses storage or protocol boundaries, distinguish the responsibilities before deciding whether representations can share a type.

| Representation | Design/review focus |
| --- | --- |
| Domain Entity or Value | Identity or value equality, relationships, valid construction, invariants and meaningful methods/allowed changes |
| Persistence record/schema, where storage exists | Keys, cardinality, constraints, lifecycle/versioning and reconstruction of domain meaning; identify which invariants storage enforces |
| Transport DTO or event, where a protocol exists | Publicly promised fields, identifiers, serialization/absence semantics, validation and compatibility for consumers |

Map authoritative facts and conversions between these representations, including relevant loss of precision, missing values and update ownership. A database schema does not by itself express every domain rule, and a generated DTO does not define entity behavior. Reuse types when meaning, lifetime and compatibility coincide; separate them when those responsibilities differ, rather than requiring three models everywhere.

Connect entity operations to the [Service and consumer model](./engineering-design.md#public-contracts-and-domain-meaning). Keep important relationships, invariant owners and conversion boundaries in the relevant [model diagrams](../documentation.md#architecture-and-interface-diagrams), with concise notes for unresolved mappings.

## Definitions, uses and observations

When a model describes reusable structure or research evidence, distinguish the following meanings before sharing a type or identity:

| Kind | Owning fact and invariant |
| --- | --- |
| Definition | The reusable responsibility, structure or behavior being authored |
| Placement or use | One use in an owning composition, with its own identity, selected definition revision and local configuration |
| Runtime instance or attempt | A concrete execution/lifetime under the actual runtime owner |
| Observation | What was captured at a stated source, identity, state and time, including coverage and missing evidence |
| Interpretation | A proposed semantic claim supported by sources, with assumptions and unresolved alternatives |
| Derived result | A projection of selected owned facts, with its scope, snapshot, omissions and refresh policy |

A definition reused twice has two uses; changing one use must not silently change every reference. A captured region does not become an authored component definition merely by renaming it. Two observations at the same address can represent different states. A saved index or report must not become an independently writable mirror of its sources. Add these distinctions only where the task's identity, lifecycle or promise needs them.

For an affected field, trace meaning and grain, writer/source, identity/scope/revision, units/default/absence, relationships, derivation/freshness and unavailable/stale/partial behavior. Keep precise declarations with the original owner. Zero observed items differs from unknown or not scanned. Do not backfill an unobserved historical fact from a current default.

Inspect both the accepted file/wire input and its normalized output before reusing a native type. A parser can turn keyed entries into an array; serializing that output may not produce valid input. Own the encoder, preserve omission/defaults and verify the affected read → edit → save → reread boundary. [Native source-chain guidance](contract-modeling/model-and-contracts.md#keep-one-native-source-chain) supplies the focused method.

## State transitions

Start from concrete scenarios: identify the actor, requested command, resulting facts or events, and the domain that owns the change. Commands express intent and may fail; events describe facts that occurred. If the model is uncertain or has many interacting cases, derive it progressively with scenario lists, state tables, typed stubs, or a local `.sketches/` note before committing to the final state model. This can be a short event-flow sketch rather than a formal workshop or messaging architecture.

| Model element | Define |
| --- | --- |
| State | Meaning, state-specific data, invariants, initial state, and terminal states where applicable |
| Trigger | Command or event, payload, and source |
| Transition | Source state, guard, next state, data changes, and effects |
| Unaccepted input | Whether an invalid, duplicate, or stale trigger is rejected, ignored, or handled idempotently |

Conceptually, `transition(state, trigger) -> Accepted(nextState, effects) or Rejected(reason)`, where state includes relevant domain data. For invariant `I`, every accepted transition must preserve `I(state) => I(nextState)`. An explicitly ignored input leaves state unchanged. This is a reasoning model, not a required API or document schema.

- Keep transition decisions and guards explicit in the owning operation or transition table. Separate these decisions from executing I/O effects; feed asynchronous results back as explicit inputs. A requested effect is not evidence that it succeeded.
- Model failure, cancellation, retries, and late results when the scenario includes them. When concurrent changes or external effects matter, specify how state acceptance and effect execution preserve consistency; a type or diagram alone provides no atomicity guarantee.
- Use hierarchical states for shared lifecycle behavior and parallel regions or separate machines for independently evolving dimensions. Define their interaction constraints rather than flattening all combinations into one enum or duplicating state across owners.

For example, a result observer may have the following illustrative transitions. This table describes observation, not the background run's lifecycle:

| Source | Trigger and guard | Next state / data | Effect and unchanged facts |
| --- | --- | --- | --- |
| Idle or unavailable | Read requested; a target is selected and current read authority exists | Reading, retaining the target and request generation | Request a read through the authority-owning service; do not start or restart the run |
| Reading | Response received for the same target and active generation | Showing the returned run outcome | Display that observation; do not infer a newer run state |
| Reading | Read fails for the active generation | Unavailable, retaining target identity | Expose inability to observe; do not mark the run failed |
| Any | Observer closed | Closed | Release owned observation resources; leave run lifetime unchanged |
| Any | Response belongs to an old target/generation or a closed observer | Unchanged | Ignore the stale response |

The real owner must define any further events its scenario requires, including authority withdrawal. A response reporting a failed run and a failure to read the run are different facts. Keep one canonical transition definition or table; a diagram explains it, and checks exercise it. The example does not supply an implementation or mandate a state-machine library.

### Effect boundaries and unknown outcomes

When one transition spans processes or an external effect, identify who grants permission, who can perform the effect, who observes completion and which boundary is actually atomic. Split the step where a crash, lost reply or independent participant can distinguish those facts. For example, `grant -> external effect -> acknowledgement` permits a crash after the grant but before the effect, or after the effect but before its reply. Merging these into one step can remove the very failure being checked.

Ask who can prove that the effect did not occur and whether an issued permission can still be withdrawn. Establish the exact identity, scope, time and completeness of the records supporting a negative fact, and whether another participant can invalidate it after observation. Timeout, lease expiry or a missing receipt alone does not retract an issued permission or justify another attempt. Use an actual exclusion, idempotency, observation or cancellation contract where it supplies the required guarantee; otherwise preserve an unknown outcome and its recovery limits.

When recovery uses retained results, distinguish the committed effect, its retained receipt and the caller's knowledge. Deleting a receipt does not undo the effect; a missing record, refusal or lost response does not establish that nothing committed. If a change alters how long recovery is promised, [identify the policy applying to old and in-flight work](collaborative-modeling.md#compare-changed-promises) before changing retention or retry behavior.

For permission-sensitive actions, distinguish “check before acting” from “permission remains valid when the action is admitted.” The latter is a stronger promise: if it is required, identify the authority that orders revocation with action admission, or an explicit grant whose validity rules supply that guarantee. Model the intervening revocation when check and use are separate. For data disclosure, distinguish admission to the controlled response channel from remote receipt; later revocation does not retract data already disclosed. Do not silently strengthen an existing contract to immediate revocation across processes.

For a finite model, retain the relevant success and unknown paths. Challenge a proposed retry or takeover rule with a trace that loses the reply after an effect; keep any observed violation distinct from a deliberately constructed bad variant. A checker result covers the declared abstraction. The real adapter must separately establish the relationship between permission, effect and observation.

At implementation handoff, map each consequential atomic step to an actual consistency mechanism and owner. A model that commits an effect and receipt together, or serializes a permission check with admission, assumes that boundary; it does not implement it. If the chosen storage or host cannot supply the guarantee, split the step and revisit the property or choose another mechanism. Retain the assumption as unverified until the real boundary has appropriate evidence.

## A progressive modeling workflow

For a complex lifecycle, derive the model in stages. Keep intermediate artifacts in the nearest owning module or directory's `.sketches/` area when that helps expose unknowns, then promote stable results to its `.docs/`, canonical contract, or task record. Do not force every project to create every artifact.

1. **Collect scenarios.** List actors, commands, external events, normal paths, rejected inputs, retries, cancellation, timeouts, duplicate delivery, process restart, and late results. Name the domain that owns each change.
2. **Name states and facts.** Define what each state means, the data valid only in that state, initial and terminal states, ownership, and invariants. Split independent dimensions instead of producing one enum for every Cartesian combination.
3. **Separate commands from events.** A command expresses requested intent and may be rejected; an event records a fact that occurred. Keep external results as explicit events or inputs rather than treating a requested effect as success.
4. **Build a transition table.** For each `(source state, trigger)`, record the guard, next state, data changes, emitted events/effects, and behavior for invalid, duplicate, or stale input. Mark unknown or unresolved cases instead of silently inventing transitions.
5. **Choose a representation.** Use the smallest form that preserves the semantics: a typed reducer or transition function for a small machine, a state diagram for communication, an executable statechart for hierarchy/timing/actors, or a temporal model for concurrency and distributed guarantees.
6. **Define effect ownership.** Keep transition decisions deterministic where possible; execute I/O, timers, queues, and persistence in an application/effect layer. Specify whether effects are at-most-once, retryable, idempotent, compensating, or only requested. Define the composition point and failure handoff.
7. **Define persistence and recovery.** Identify which facts must survive which failures, when they must be saved relative to effects, and the required concurrency/atomicity guarantees. Then choose memory, files, a database or a workflow store through [technology selection](engineering-design.md#technology-selection-design). Recovery does not by itself require a database. Specify relevant versioning, serialization, restart behavior and handling of unknown effects.
8. **Verify and refine.** Check reachability, dead ends, completeness of important triggers, invariants, rejected paths, duplicate/stale inputs, cancellation, timeout, restart, and concurrent sequences. Revise the model before patching implementation when a new case reveals a missing or ambiguous state.

Useful intermediate artifacts include a scenario inventory, state vocabulary, transition table, event/command catalog, invariant list, Mermaid diagram, typed skeleton, fixture or trace set, and a short decision note explaining rejected alternatives. A document may combine these; they are aids to reasoning, not a required template.

Use the selected model as part of the [method working record](./project-context.md#make-method-execution-explicit): distinguish examined transitions/invariants from unresolved cases and retain the next modeling action. For interface or protocol design/review, include the relevant [behavioral diagram](../documentation.md#architecture-and-interface-diagrams); do not create a second progress document when the model or task already records that information.

## Representations and tools

Select a tool by the property that needs to be expressed. Verify current project dependencies and official tool semantics before adopting one; a tool is not a substitute for an owned domain model.

| Need | Suitable representation or tool | Boundary and caution |
| --- | --- | --- |
| Explain states and flows to humans | Transition table, Mermaid `stateDiagram-v2`, sequence diagram | Documentation only unless separately connected to executable checks; keep guards and edge meaning explicit |
| Small deterministic domain lifecycle | Typed discriminated union plus `transition(state, trigger)` or reducer | Keep effects outside the pure transition where possible; test accepted and rejected inputs |
| Hierarchy, parallel states, actors, guards, timed transitions, or executable inspection | Statecharts; an established implementation such as XState, or an SCXML-compatible runtime when the project needs that standard | Adopt only for actual complexity; keep domain semantics independent of framework-specific machine objects |
| Tool-neutral statechart interchange or integration | SCXML (State Chart XML) | Treat it as a protocol/representation with version and runtime compatibility concerns, not automatically as a domain model |
| Concurrent or distributed safety/liveness properties | TLA+ or PlusCal, with model checking where practical | A verification model is not production code; record the modeled scope and assumptions |
| Finite structural constraints across entities and relationships | Alloy or a comparable constraint model | Useful for exploring possible configurations; it does not define effects, persistence, or runtime scheduling |
| Durable, long-running workflows with timers, retries, and restart recovery | A mature workflow engine such as Temporal, if its operational model fits | This is an infrastructure choice, not a default replacement for a local state machine; evaluate deployment, persistence, retry, and compatibility costs |

Keep a canonical semantic model even when several representations exist. A Mermaid diagram may explain a statechart, a typed contract may implement it, and a TLA+ model may check concurrency assumptions; link them to one owner and label generated or illustrative copies. Do not maintain conflicting hand-edited diagrams, schemas, and machine definitions without a clear source of truth.

## From model to implementation and checks

Map each stable state to a representation that makes invalid combinations difficult to express, such as a discriminated union or state-specific record. Expose commands and results through the owning domain contract. Keep guards and transition decisions in one operation or machine definition; keep adapters responsible for transport, storage, timers, and external effects. If a state machine crosses a module boundary, expose commands, events, or query results rather than the machine's private context.

At minimum, test every important accepted transition and representative rejected input. For a non-trivial machine, add table-driven or model-based tests generated from the transition table, property-based checks for invariants, and traces covering duplicate, stale, concurrent, cancelled, timed-out, restarted, and unknown-outcome paths as applicable. Verify persistence and integration contracts separately from pure transition logic. Type checking or a rendered diagram is evidence of structure, not proof that all event sequences and effects are correct.

State machines do not imply event sourcing, an event bus, CQRS, a repository for every entity, or a workflow service. Add those mechanisms only when durability, auditability, scale, or integration requirements justify them.

## Apply proportionately

Record only the useful model in the existing design artifact: types, a transition table, a short event flow, or a Mermaid diagram. Link canonical declarations rather than maintaining competing versions. A typed function or small transition table is often enough; use a mature state-machine library when hierarchy, timing, or orchestration warrants it, following [reuse guidance](./engineering-design.md#reuse-before-building). Domain events do not require an event bus, event sourcing, or a repository for every entity.

The implementer checks the affected transitions and invariants, including relevant rejected inputs and failure cases. Type checking or enumerating states does not prove all event sequences, timing, or external effects correct; choose focused checks for the actual behavior and risks.
