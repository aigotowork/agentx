---
schema: scenario
description: Trace complete caller data supply and choose honest evidence from choreography, scripted fixtures or finite specification Mocks
tags: [contracts, scenarios, mocks, evidence]
---

# Caller flows and Mock evidence

Use this method to connect a canonical user scenario to native operations and inspect its complete result under the assignment's limits. Compose shared [collaboration and data supply](../collaborative-modeling.md#trace-collaboration-and-data-supply) and [effect/unknown-outcome modeling](../data-state-modeling.md#effect-boundaries-and-unknown-outcomes); those sources own the underlying rules.

## Trace a complete outcome

Select the required scenario IDs and criteria from the existing scenario source. Cover the normal path and the decisive rejected, conflict, missing-input/decision, partial or unknown branches that affect its promise. Reuse these IDs in flow scripts and checks rather than rewriting a second requirement catalog. A simulated path that stops at “request accepted” is incomplete when the scenario requires observation, review, continuation or a deliverable.

For each meaningful step, record the following in the existing contract topic, scenario case or typed caller file. Use a short table or nearby comments; no universal registry is required.

| Caller question | Required connection |
| --- | --- |
| What is being requested? | Scenario criterion → owning native operation; identify caller and authoritative actor context |
| Where does each input come from? | Caller input, prior result, permitted query, trusted context or declared fixture; identify the actual source and required conversion |
| Do the facts refer to the same work? | Project/resource/use-site identity, scope, baseline or fixed frontier, capability-key namespace and correspondence between calls |
| Are the facts usable now? | Version agreement, freshness/expiry, authorization/admission condition and owned/borrowed resource lifetime |
| What permits the next action? | Discriminated success/acceptance/refusal/decision/unknown result or event; handle consequential variants explicitly |
| What changes and what is retained? | New facts/revisions/effect intents and unchanged unrelated objects, drafts, history or observation targets |
| How can work continue? | Stable work/operation/decision identity, allowed continuation/query and who owns recovery |

Check all required facts, including an operation's hidden dependency on configuration, capability or a retained receipt. A valid fixture is an explicitly supplied premise, not proof that a real operation can supply that fact. Mark a missing source or unsupported conversion as a gap. Two schema-valid reads do not establish compatible revisions, scopes, authority or key namespaces. Do not replace a missing fixed-version read with “latest” or change a capability key solely to make the flow pass.

Treat a required user decision as an owned request and response: question/options, affected scope, baseline, continuation identity, expiry or supersession when applicable. A reply resumes the same work only after its owner accepts that request/version/authority; it is not an implicit new run or a blind repeat of the last action. These are modeling questions, not mandatory product names.

## Select the evidence that answers the question

| Evidence | Permitted mechanism | What it establishes | What remains unverified |
| --- | --- | --- | --- |
| Compile-only choreography | A typed caller accepts the real contract-derived client/port; it is compiled and never invoked | Expressed inputs/results compose and consequential branches can be represented | Runtime parsing, legal transitions, data availability, concurrency and effects |
| Schema-valid fixture / scripted Mock | Parse explicit finite fixtures; return selected responses for declared calls | Fixture shape, chosen response handling, requested call sequence and recorded arguments | State guards, actual supply completeness and general behavior outside the script |
| Finite specification state model / Mock | In-memory deterministic state, explicit guards/transitions and declared external-result events | Selected invariants and transitions for the explored finite inputs/traces under stated assumptions | Real atomicity, durability, authorization machinery, transport and provider guarantees |
| Actual frontend consuming a Mock | Run the real UI through its shared client and selected Mock host; retain observed actions, responses and resulting view state | The observed UI path obtains facts, handles selected results and maintains continuity with that supply | Unvisited UI states, untested transports and production services/effects |
| Actual HTTP against a Mock | Run the chosen client and server adapter over a real HTTP listener; observe requests, serialization and result/error mapping | The exercised HTTP boundary and Mock behavior for the selected cases | Production authentication, persistence, provider behavior and wider protocol coverage |
| Real service evidence | Separately authorized implementation and checks at the actual boundary | Only the observed behavior with its environment, sources and coverage | Untested paths and wider guarantees |

State which kind each file/check supplies. A scripted sequence returning a conflict once is not a state model that detects stale revisions. An asynchronous function returning canned success is not a service implementation. A finite Mock is an executable specification only for its declared state, inputs, guards and properties. Reserve exhaustive claims for actually exhaustive exploration of the stated finite model; a few traces remain selected-case evidence. Browser interception may exercise a request envelope without reaching a listener; frontend evidence and HTTP evidence are separate dimensions. [Runnable Mock provisioning](../mock-provisioning.md) extends the delivery mechanism when authorized without upgrading its production guarantees.

## Keep Mock logic inside the assigned boundary

Use the same native schema and contract-derived caller types. Parse fixture inputs and outputs with the appropriate owning schemas at the simulated boundary; document transforms rather than parsing one value twice under incompatible assumptions. Reject unexpected scripted calls and validate their arguments. Do not use unchecked casts or permissive placeholder schemas to force a fixture through.

For a specification state Mock, retain initial state, valid finite domains, state/transition owner, guards, deterministic time/identity inputs and expected properties derived from the scenario. State changes and intended effects must be inspectable. Keep external outcomes as injected events, including late, duplicated, partial and unknown results where needed. Explicitly label any serial/atomic transition assumption. Do not infer production compare-and-set, durability or distributed cancellation from a single in-memory update.

Specification work includes fixture builders, schema parsing, bounded in-memory records, specified revision/scope/authority guards, finite transition functions, scripted result selection, effect logs, typed caller composition and focused case checks. In a declarations-only task these remain on a clearly named example/specification surface. When the current task authorizes runnable Mock supply and prototype integration, use [the provisioning method](../mock-provisioning.md) to share this behavior through the actual frontend client. Keep the supply explicitly identified as simulated.

A declarations-only task does not authorize server handlers/listeners or product wiring. A runnable-Mock task may authorize those mechanisms for simulated supply; choose the needed host rather than assuming a real HTTP service is required. Neither authorizes real Provider/Bot/API calls, paid requests, production persistence, infrastructure or production orchestration. Use a simulated caller context to exercise an authorization rule; building a real authentication service needs its own authorization. A specification record can model facts retained across a simulated restart; it supplies no real restart survival. Read the current task and project binding for the applicable boundary.

## Challenge the promise with deciding branches

Choose the cases that can falsify the scenario; do not create a test per schema field or every Cartesian state combination.

| Risk in the selected flow | Deciding check |
| --- | --- |
| Baseline or scope drift | Same shape with wrong resource/project/use-site or stale revision is refused; authoritative state and unrelated work remain unchanged |
| Permission/lifetime changes | Expired grant, changed authority or disposed borrowed handle follows its declared refusal/recheck rule; preflight success alone is insufficient |
| Missing fact or user decision | The flow retains original work identity and sufficient context; continuation checks the accepted decision/baseline |
| Duplicate or unknown external outcome | A lost reply retains original operation identity; observe/reconcile if supported and keep unknown otherwise; no automatic second dispatch |
| Partial success | Preserve usable outputs and the failed attempt; a new attempt names reused results and remaining scope where the native model permits it |
| Stop or disconnect | Distinguish leaving observation, requesting cancellation and confirmed termination; a late result is handled under the same operation identity |
| Projection loss | Replay the affected valid/refused branch through the proposed adapter policy; identify distinctions the wire schema cannot validate |

For unknown effects, retain the point at which dispatch may have happened and the caller lost knowledge. Do not rewrite unknown as “failed before effect,” silently roll back an effect log or claim that absence of a receipt proves non-execution. A new attempt requires the actual domain's recovery/idempotency contract, not an expedient Mock success.

Select checks under the task's actual authority. Use bounded type checks, fixture parsing and finite specification cases for declaration/specification work. For runnable Mock integration, add the actual selected frontend and transport boundaries required by the task, with execution assigned to its integration owner. No application-wide suite or provider probe follows merely from adding a Mock. Distinguish authored from executed checks and retain coverage/assumptions with results. Static walkthroughs and structural equality do not establish runtime behavior.

## Deliver at the authorized boundary

Complete the existing task with links and concise evidence answering each applicable item:

- Canonical scenario criteria connect to native objects and operations; current facts, target decisions and open assumptions are distinct.
- Existing native definitions are imported/composed or bridged with explicit JSON/value semantics; no second handwritten types or operation registry is authoritative.
- Field constraints, invariants, operation guards, stable errors/results, changed/unchanged facts, effects and lifetime have identifiable owners.
- Required flow inputs have declared sources and identity/version/scope/authority/lifetime agreements; missing supply remains a named gap.
- Normal and decisive refusal/decision/unknown branches are represented; SDK and other proposed surfaces preserve their distinctions or disclose losses.
- Every check identifies declaration/compilation, fixture script, finite specification, actual UI, HTTP or production-boundary evidence as applicable, with executed coverage and unsupported guarantees.
- Future enforcement obligations name the owning domain/service, mapper or consistency boundary and the evidence still needed; Mock-backed prototype behavior is identified separately from production support.
- The authorized files and discovery routes are ready for the next owner, with paths, limits and released write scope. Required product-model/catalog updates go to their assigned owner.

Stop when the assigned declaration or runnable-Mock result is reviewable against its task. Do not implement a production service merely to remove an explicit evidence gap. If a promise cannot be supplied without changing the domain boundary, return that concrete conflict and alternatives to its owner while continuing independent work.
