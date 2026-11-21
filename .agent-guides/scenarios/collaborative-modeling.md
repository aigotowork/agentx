---
schema: scenario
description: Derive and refine module responsibilities and contracts through scenarios, explicit models and bounded checks
tags: [domain-modeling, contract-design, design-verification]
---

# Scenario-driven collaborative modeling

Use for a selected capability, module or cross-module behavior whose meaning, responsibility or contract needs derivation. Enter at the affected step for a bounded change. For an uncertain whole product, [product design](product-design.md) owns needs, scope and cross-stage decisions; pure presentation work uses its [web method](../domains/webdevelopment/AGENTS.md). Reuse professional definitions in [engineering design](engineering-design.md) and [data/state modeling](data-state-modeling.md); this scenario owns the working loop and handoff.

Stay within the assignment. Begin with readable domain documentation and native contracts; implementations can consume those same declarations. Select compile-only choreography or an isolated verification model only for a question it can answer. A verification model contains specification logic; a type stub contains no provider behavior. Prototype and implementation work require their actual task scope. Keep progress and useful intermediate results in the existing [method record](project-context.md#make-method-execution-explicit), not a parallel tracker or compulsory artifact set.

For simple content, express it directly. For new models, complex relationships, interacting states or multiple dimensions, apply [visual model communication](interactive-design-communication.md): derive a polished panoramic HTML view from the domain model, compose professional visual forms such as Mermaid, SVG, Canvas, chart libraries or 3D renderers, deliver the portable componentized HTML file early; when shared access is needed, serve that same file through LAN HTTP and update it with the canonical model. This also applies to systems without a frontend.

## Select the starting evidence

| Starting point | Action before relying on the model |
| --- | --- |
| New capability | Identify an actor, desired result, deciding scenario and important failure; label assumptions |
| Existing implementation without a usable model | For substantial reconstruction, establish the [whole-scenario map](product-design.md#enter-at-the-current-question), then trace a selected scenario through public declarations, callers and evidence; retain source era, fact and implementation owners, declared/implemented/installed/exercised support and investigated coverage; observed behavior is not automatically a requirement |
| Semantic change against an existing model | Check affected source freshness and [compare the candidate's promises](#compare-changed-promises) with the baseline; retain historical evidence and separate intended changes from corrections of the baseline interpretation |
| Correction or refactoring with unchanged semantics | Reuse the affected scenario, model and native declarations; edit the original material and check the affected guarantees without reconstructing the model. Reclassify the work if identity, defaults, outcomes or other promises change, even when signatures do not |

Use user requirements, accepted decisions, observed facts, supplied fixtures and unresolved assumptions as distinct inputs. Do not silently substitute current source behavior for a future proposal, or a proposal for current behavior. When independent reconstruction or review is explicitly selected, preserve a fixed, source-bound baseline for that work; ordinary modeling does not require another reviewer.

Keep concept meaning, responsibilities and behavioral rules in the owning domain topic or existing declaration comments. Use prose, diagrams and tables where they explain the selected question; keep exact types, fields, signatures and command grammar in their native source. Link or import those declarations instead of maintaining design-only mirrors. Tools derive extractable structure from these materials; leave meaning they cannot extract in the original documentation. A hand-maintained object registry or projection script is not a prerequisite for a usable model.

Follow linked methods only for the unresolved question and reuse their established results. Product design supplies the scenario promise, this method derives its collaboration, and engineering/data-state methods supply precise boundary rules; these are not recursive prerequisites. Finite checking is optional and belongs to the selected check below, not the entry cost for defining a domain concept or a small interface.

### Retain the needed basis

For current document iteration, use canonical links and the affected diff. If concurrent edits make a conclusion's basis ambiguous, record the affected sources and uncertainty; capture reading ranges, time or byte identity only when the conclusion needs that distinction. For a fixed machine report that must remain explainable, retain the selected model/input and necessary rule bytes, or an immutable version from which those bytes can actually be retrieved. Expand this to the receiving reader's required source closure for an offline handoff. Ordinary modeling needs no source register or snapshot package. Do not freeze the whole repository by default or copy the entire methodology into each product.

A suitable source helper is optional. Git HEAD alone does not identify dirty working bytes; a digest proves identity, not semantic correctness or that every line was examined, and cannot restore missing bytes. On drift, preserve the original report and inspect affected assumptions before selecting a new basis. Updating a digest does not extend an old result to changed behavior.

### Compare changed promises

For a consequential contract change, compare the original and proposed observable guarantees even when signatures are identical. Identify the affected caller/provider or message direction; for a protocol boundary, use [API/SDK guidance](api-contracts-and-sdks.md) for structural compatibility. Separately inspect changed permissions, ordering, recovery, retained data and result meaning against the original scenario. A newly weakened property passing its checks does not establish compatibility with the old promise.

When a policy changes over time, identify which promise applies to existing consumers, stored facts, issued permissions and in-flight operations. State the policy's effective boundary and, for time windows, the starting event and endpoint semantics. Assign the decision to preserve the old guarantee, coordinate adaptation or explicitly change the promise to its actual owner; updating a new client does not migrate old work. Keep unexamined consumers and unsupported implementation assumptions visible, preserve historical evidence, and recheck only affected obligations.

For recovery across upgrades or rollback, preserve the original intent's identity and policy association, or specify an explicit migration. A new route must not silently create a second execution identity for the same intent. Account for late completion before retiring old recovery data or recognition paths; elapsed time since deployment alone does not establish that old obligations have ended.

## Establish the scenario and next uncertainty

Start with who wants what, under which conditions, and how completion or failure can be observed. Include what must remain unchanged. If the goal is still disputed, retain that question and return it to the product owner rather than inventing success data.

Reuse the selected [scenario nodes and criteria](product-design.md#maintain-one-scenario-source) as the input: derive the facts and guarantees each criterion needs, assign their owners, and trace their collaboration. Return gaps and result semantics to that source instead of creating another version of the user promise.

Choose the uncertainty that can change the boundary: concept meaning, ownership, collaboration, required data or technical feasibility. Compare the smallest useful alternatives with their tradeoffs and assumptions. Examine available capabilities before proposing mechanisms, following [technology selection](engineering-design.md#technology-selection-design). A consequential external assumption needs a bounded experiment or an explicit unresolved dependency, not a promise based on a library name.

Use examples and prose while meaning is unsettled. Structure the part whose identities, outcomes and relationships can be stated without inventing answers; directly edit clear existing declarations. Keep remaining design questions visible. A design unknown is different from a caller's unknown result after an attempted effect.

## Derive concepts and behavior

Apply [data and types](data-state-modeling.md#data-and-types) to stable identities, values, relationships and invariant owners. Roles, policies, facts and capabilities can be useful distinctions without each becoming an entity, class, service or package. Separate domain meaning from storage, transport and UI representations. A historical fact, current resource state and observer knowledge may need different lifetimes.

For a consequential invariant, state its scope, owner, violating example and observable consequence. When input meaning depends on state or history, use [state transitions](data-state-modeling.md#state-transitions): distinguish command intent, occurred facts, legal transitions and effect intentions. Keep current authority and exact resource identity with their owner. Cancellation of waiting does not itself establish cancelled work; a missing reply does not establish that nothing happened.

Try deleting or merging a questionable concept and replay the deciding scenario. Keep a distinction when identity, lifecycle, authorization, history or consistency would otherwise be lost. Absence from the selected happy path does not justify deletion when recovery, constraints, compatibility or known external uses require it. Unsupported future possibilities need not keep speculative abstractions in the current model.

## Trace collaboration and data supply

For native procedure contracts, use [caller flows](contract-modeling/caller-flows.md#trace-a-complete-outcome) to trace input, identity, revision, decision and continuation supply. For a runnable frontend simulation, select [Mock provisioning](mock-provisioning.md). A schema-valid value has a shape; it does not establish who supplies it or that independently valid values compose.

Trace reception, decision, required facts, state change, intended effects and reported outcome through the relevant participants. Specify accepted, rejected, partial and unknown outcomes where they matter, with retained state and recovery ownership. Include frame conditions: which objects and facts this action must leave unchanged. Name the consistency boundary rather than relying on separate callers to preserve one shared invariant.

Keep public [contract semantics](engineering-design.md#public-contracts-and-domain-meaning) canonical. Import existing declarations where their meaning fits; label proposed bindings. A compile-only scenario can expose required parameters and result branches; inspect consequential variants explicitly. It does not prove timing, legal transitions or real provider effects. Never return a scripted success as evidence of an unimplemented guarantee.

Trace each necessary consumer fact back through provided operations, projections and permitted transformations to its source. Declare all required inputs and actual alternatives. Where the scenario depends on them, record the owner and agreement condition for identity/scope, revision or fixed frontier, lifetime/freshness, permissions and callable-key namespace at each boundary. Check that independently valid reads can be composed at the required version and that an advertised capability key is accepted by the actual invocation path. Provider existence or matching field names prove neither condition. Mark unexamined providers, conversions and dynamic paths unknown; do not silently substitute a newer frontier or reinterpret a key to make the mapping pass. Use [representation mapping](data-state-modeling.md#domain-storage-and-transport-representations) for storage/transport differences, and [API/SDK guidance](api-contracts-and-sdks.md) for a real protocol boundary.

For ordering or concurrency, state the environment, interleavings and atomic steps. Derive properties from the user promise independently of the tested guards. An action that must not stop a resource does not prevent the environment from independently ending it. A final state cannot alone establish a required earlier event order; retain the relevant history or a labeled observation in the verification model.

When a transition crosses a process or external effect, apply [effect boundaries and unknown outcomes](data-state-modeling.md#effect-boundaries-and-unknown-outcomes) before choosing atomic steps. The same rule applies when eligibility depends on proving that an action has not happened; missing observations alone do not establish that fact.

### Carry a promise across interfaces

Where a scenario crosses CLI, UI, module or protocol boundaries, explain the relationship between the original promise and each interface. Keep [command behavior](command-line-design.md#results-effects-and-recovery), [product interaction](../domains/product-design/requirements.md#define-feature-and-interaction-promises) and [native code contracts](engineering-design.md#contract-boundaries-and-integration) with their respective owners.

| Relationship | Trace only what matters to the promise |
| --- | --- |
| Use | Which domain operation supplies the action or read, and where its required inputs come from |
| Refinement | How a scenario or domain rule becomes precise at this boundary without changing its meaning |
| Composition | Which capabilities jointly deliver the result, with an owner for ordering, consistency and recovery |
| Projection | Which facts become visible and which identity, state, permission or outcome distinctions must survive |
| Local convention | What the interface itself adds, such as exit codes, focus or return navigation, without implying a stronger domain guarantee |

For example, a promise that leaving a report view keeps its run alive constrains both CLI interruption and UI navigation. Each observation adapter releases its own resources; the run owner retains the stopping authority. The CLI owns its exit behavior and the UI owns its return path. Neither may convert an unavailable read into a failed run. These are several expressions of one domain rule, not a requirement for identical commands, buttons and Service methods.

Record a compact mapping only when ordinary links or an existing sequence view do not make the relationship clear. Reference the original criterion, domain rule and native declaration rather than copying their fields. Replay the deciding success and failure across the affected boundaries; retain unexamined consumers and unsupported mappings explicitly. This requires no universal contract schema or per-field registry.

## Select and interpret checks

Choose only checks that answer the current question. Read the selected tool's owned usage and public contract for its actual version, inputs, invocation, output destination, side effects and bounds. An unavailable tool leaves a named gap; continue labeled manual reasoning or choose an available suitable method. Tool installation, a research checkout and conversion of all prose to TypeScript are not prerequisites for design.

| Question | Minimum useful material and check | Interpretation |
| --- | --- | --- |
| Do contracts compose? | Canonical declarations, compiler configuration and a bounded compile-only consumer | Type diagnostics cover expressed relationships, not business validity or runtime effects |
| Can the selected consumer obtain required facts? | Source-located supply mappings, necessary inputs and coverage; use a supported mapping checker | A proven missing source under complete declared coverage is a violation; partial coverage leaves unknown; no automatic code/storage lineage |
| Can event sequences break a promise? | Initial state, finite domains, transitions, independent properties/goals, environment and atomicity assumptions | Retain counterexamples and success/rejection reachability; missing goals or disabled actions can make safety pass without fulfilling the use case |
| Is a query plausible for this result and workload? | Result granularity, keys/cardinality, query/schema and workload with a selected database's result/plan tooling | A plan is bounded evidence; join count alone is not a domain defect |
| Does a changed baseline still support the model? | Referenced source versions, semantic differences and affected claims | Source drift requires review; a matching hash proves byte identity, not correct interpretation |

With an explicit finite-model or supply checker, retain assumptions, source/model/tool versions or digests, coverage and budgets. Separate execution completeness from findings: complete analysis can report violations, and incomplete analysis can retain a valid counterexample. Source changes or unavailable sources must not become a fresh pass; review the mapping before updating the basis. Treat missing/invalid input and checker failure separately from a design violation.

When adapting a semantic projection to checker input, retain both versions and map original operations, transitions and properties to the derived input. Record assumptions, finite domains, abstractions, split or merged steps and omitted behavior. Keep a property coverage map: mechanically checked (input property and report), manually reasoned (argument and source), or uncovered (gap and consequence). State-only checks do not prove edge or temporal properties; property counts do not establish coverage of the original obligations. Read the selected tool's adaptation guidance when this translation is needed. Input hashes and a passing check do not prove fidelity to the projection or implementation.

Decline a check whose supported relation cannot answer the question. An all-of supply check may establish provider availability under declared premises; it cannot establish revision or callable-key agreement merely by treating those conditions as given. Retain the specific manual argument or uncovered obligation instead of manufacturing a green result.

For bounded exploration, keep discovered/processed states, remaining work and the cutoff reason. No violation before a cutoff is not exhaustive success. If paths or traces are clipped, retain their completeness and total-length indicators; a retained prefix is not a full witness. Existential goal reachability does not prove fairness or inevitable completion. Tool-specific limits and exit codes belong in its documentation, not a second command contract here.

Judge meaning alongside mechanics: does the observed result still satisfy the intended outcome, and do the abstractions represent the relevant environment? Record each issue with trigger, violated expectation, source, evidence and disposition. A synthetic bad model tests the checking method; it is not a discovered production defect.

## Repair and hand off

Route the issue to its owner: need, domain rule, public contract, data mapping, query/storage design or environment assumption. Keep a concrete alternative when a boundary cannot deliver its promise. Do not invent fields or weaken a property merely to clear a diagnostic. Preserve meaningful before/after differences and recheck only affected inputs; reuse a passed check when its inputs and assumptions remain unchanged.

Use the same scenario, model and native contracts to compare proposals, refine the selected design and organize implementation work through [design management](../domains/design-management/workflow.md#represent-a-planned-change). Put established semantics into their owning topic, so the next maintainer can read the current model without reconstructing it from successive Task notes.

The handoff must connect the parent scenario to owned operations and facts, neighboring capabilities, success/rejection/unknown behavior, state and identity constraints, effects and recovery, relevant evidence and unresolved dependencies. Leave internal algorithms free where the contract permits. A usable declaration does not establish integration; return results to the same user outcome through [product design](product-design.md#deepen-modules-and-return-their-results).

### Assign implementation obligations

For each consequential guarantee, connect the existing scenario criterion and canonical contract to the owner that can enforce it. Keep the connection in the existing contract or task; a separate obligation registry or fixed record format is unnecessary.

| Handoff question | Useful answer |
| --- | --- |
| Who can preserve the guarantee? | The operation and consistency boundary owning the required facts, or the explicit coordination contract between owners |
| What realizes it? | The actual domain decision, application coordination, storage, transport or host boundary; separate mechanically generated bindings from behavior that must be implemented |
| Which assumptions remain? | The required ordering, atomicity, durability or authority condition, its proposed mechanism and responsible owner; use [effect boundaries](data-state-modeling.md#effect-boundaries-and-unknown-outcomes) where relevant |
| What would support completion? | A deciding success/failure trace and evidence at that boundary, with the source basis and current scope; distinguish supplied evidence from required but unavailable evidence |

An implementation obligation is still open when its only support is a typecheck or a finite model that assumes the mechanism. The implementer must establish that mechanism with appropriate evidence, revise the design, retain a named unresolved condition, or return an infeasible promise to its owner. Keep internal choices free where they cannot change the public guarantee. Select generation and integration through the existing [contract/SDK route](api-contracts-and-sdks.md), without retyping its declarations or commands here.

Reconcile the handoff with the original contract as well as the selected model. Required behavior omitted from an abstraction, such as durable event delivery, remains an obligation with its original semantics and evidence needs; omission is not permission to remove it.

Finish when the selected question has an actionable result and the next owner can distinguish required guarantees, implementation freedom and unverified assumptions. State what was reasoned, declared, mechanically checked and actually exercised. After separately authorized implementation, use its evidence to establish a new current baseline; do not relabel an accepted proposal as implemented behavior.
