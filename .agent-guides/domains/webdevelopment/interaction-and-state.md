---
schema: scenario
description: Own frontend state, effects and transitions while preserving user context and recovery
tags: [web, interaction, state]
---

# Interaction and State

Use this guide when designing meaningful states/events at any component level, or working on forms, navigation, asynchronous data, global/URL state and state-related bugs. The [general state guide](../../scenarios/data-state-modeling.md#state-transitions) owns lifecycle modeling and invariants; this guide connects that model to visible frontend behavior. Use an explicit machine when its [selection criteria](../../scenarios/data-state-modeling.md#when-to-use-a-state-machine) apply, not for every toggle.

## Component and page protocols

Treat publicly relevant inputs, actions/events, states and result guarantees as an interaction protocol. This applies within the frontend as well as across frontend/backend boundaries; the [general contract method](../../scenarios/engineering-design.md#contract-boundaries-and-integration) does not require a network transport.

| Boundary | Express in its protocol |
| --- | --- |
| Parent and Component/Block | Typed inputs, requested actions, authoritative state confirmation and observable results/failures; keep private child state and DOM details local |
| Page/Screen workflow | Entry conditions, permitted user actions and step/navigation transitions, completion, cancellation, recovery and retained context where relevant |
| Frontend domain capability | Service operations, owned data/state, results/events and effect guarantees; add a transport contract only when the capability crosses that boundary |

During design or review, create or update the relevant [interface, interaction or state diagram](../../documentation.md#architecture-and-interface-diagrams) alongside the existing component/page contract or spec. Use a structure/type view for public relationships, a sequence/flow view for cooperating actions, and a state view when lifecycle rules matter. Record which behavior has been confirmed and what remains unresolved through the [working-record method](../../scenarios/project-context.md#make-method-execution-explicit).

A callback request is not confirmation that its effect succeeded. Read confirmed state from its authority, model asynchronous outcomes where needed, and map that behavior to the UI using the model below. A simple input-to-view component needs no invented lifecycle or formal state machine.

## Model behavior before presentation

For a non-trivial Page, Screen, Block or Component, use **data sources and ownership → states and events → transitions/effects → view projection → implementation**. Start from the task's behavior before detailed visual design or JSX. A simple presentational component can remain a projection of its inputs; this perspective does not require local state, a machine library or a separate model file.

Keep a compact model with the component's existing contract/spec. Use the smallest useful expressions:

| Part | Concise expression | Establish |
| --- | --- | --- |
| Data map | `fact: type ← source; owner; update/reset rule` | Props, event payloads, shared/remote snapshots, local state and derived values; identify the authoritative writer |
| Behavior model | `state + event [guard] → next state + effects` | Meaningful states, event origin/payload, accepted transitions and relevant failure/cancellation behavior; follow the [general transition model](../../scenarios/data-state-modeling.md#state-transitions) |
| View projection | `state + current input/data → regions, variants, available actions, feedback` | How the established behavior appears, including focus, motion and recovery where relevant |

Separate lifecycle modes from ordinary data: every possible text value is not a new machine state. Keep independent dimensions separate. Model only this component's responsibility; coordinate children through their public events and observable guarantees instead of flattening private child machines into the parent.

For example, one slice of an editor design can be expressed as:

```text
data: record <- parent props; draft <- locally owned; canSave <- permission selector
derived: dirty <- compare(draft, record); valid <- validate(draft)
logic: editing + submit [dirty && valid && canSave] -> saving / requestSave(draft)
view: saving -> retain editor and draft; indicate pending; prevent duplicate submit
```

This is an illustrative slice, not a complete editor model. Add the relevant result, error and external-update transitions before implementation. UI exploration can expose missing behavior; revise the model rather than encoding new business rules only in presentation conditions.

## State ownership

Give each authoritative fact one owner. Derived views can read or calculate from it; independently meaningful facts may have different owners.

| Source or state kind | Typical owner | Boundary decision |
| --- | --- | --- |
| Parent props or controlled values | The parent or capability supplying them | Read current inputs; emit requests through the contract and define relevant replacement/reset behavior instead of maintaining a competing writable copy |
| Transient presentation | The smallest component that needs it | Opening, hover or local disclosure usually does not require global storage |
| Form draft or small task lifecycle | The Component/Block performing the task | Define reset, validation, submission and preservation on failure |
| Selection or coordination shared by children | Their nearest meaningful common owner | Lift only what the coordinated behavior requires |
| Addressable navigation context | The route/URL when refresh, linking and history should preserve it | Define encoding, defaults and back/forward behavior; keep unsuitable sensitive/transient data elsewhere |
| Remote facts and request lifecycle | The established service/query layer | Separate authoritative remote data, cached representations and editable drafts |
| Cross-page application facts | The owning application/domain context | Share because the meaning is cross-page, not because passing an input is inconvenient |
| Derived information | A calculation from canonical inputs | Do not maintain a second writable copy without a real synchronization requirement |

Keep view-state and domain-state meanings distinct. A selected tab, an unsaved draft and a persisted order status are not interchangeable merely because each is represented by a string.

An event payload supplies input to the owning transition; it is not automatically another persistent state source. Define how relevant prop, URL, shared-store and remote-data changes enter the model through inputs or explicit notifications. A controlled child can request a change while the external owner performs it; display confirmed state from that authority.

## Make actions and effects explicit

For each consequential action, identify its source, accepted states, guard, state change, external effect and user-visible result. A callback can request work; it must not imply that persistence has already succeeded.

```text
user action -> owning transition -> requested effect
            -> result associated with that request -> visible feedback
```

External results must be associated with the relevant object, request or revision when overlap is possible. Define how stale, duplicate, cancelled or failed results are handled. Avoid turning a late response into a change to whichever item happens to be selected now.

Keep Page/Block orchestration behind [capability contracts](../../scenarios/engineering-design.md#contract-boundaries-and-integration). Map transport results into the intended view/domain model. Standard controls should not acquire unrelated service, persistence or application-global dependencies.

## Design asynchronous UX

| Behavior | Decide before implementation |
| --- | --- |
| Initial load | Which regions wait, what can render usefully, and how an empty result differs from failure |
| Refresh or background update | Whether existing content remains visible and how freshness or partial failure is communicated |
| Submission | Which actions are blocked, whether duplicate requests are safe, and how progress is shown |
| Optimistic update | Which outcome is safe to show provisionally and how rejection, rollback or reconciliation appears |
| Cancellation or navigation | Whether work can be cancelled, whether results are ignored, and what state survives |
| Error and retry | Which input/context is retained, what can be retried, and whether the original effect may already have happened |
| Access or session change | How the affected task stops or recovers, without pretending a UI restriction enforces server authorization |

Use the existing request/cache mechanisms where they fit. Do not implement a new caching or state framework solely to model a few states.

For a saving editor, a small transition table might specify: submit preserves the draft while pending; success updates the confirmed version; failure keeps editable input and offers recovery; a response to an older draft does not erase a newer edit. The exact representation belongs to that editor, not a universal form-state schema.

## Interaction continuity

For a multi-surface task, specify which object/use-site/revision, selection and batch scope, unsaved draft/baseline, comparison, location, focus and pending-operation identity survive each departure and return. Restore only still-valid context; deleted, moved, stale or unauthorized targets need explicit revalidation and relocation. Returning to a draft without its selection or comparison can still lose the task.

Separate a local preview, saved draft, accepted execution, applied result and undo by their actual effect. Cancelling a wait, requesting cancellation and reversing a committed result have different contracts. Mixed or partial batch outcomes retain completed, remaining and uncertain items; do not retry the whole collection automatically. Use [interaction bindings](../interaction-modeling/AGENTS.md) when these distinctions must be traced from scenario through control and native result.

Treat focus, scroll position, selection and return context as observable behavior. Define where focus goes after opening, closing, deleting or navigating, especially when the triggering element disappears. Preserve drafts according to the user task and explain consequential loss.

For forms, connect labels, constraints, validation and errors to the affected inputs. Choose validation timing that supports correction. For destructive or costly actions, choose confirmation or undo according to consequence and reversibility, using established product conventions.

Coordinate [motion](./motion.md) with these transitions, but do not make decorative animation completion the authority for domain success. Responsive rearrangement should preserve the same task state unless a deliberate reset is part of the contract.

## Check behavior at the failure boundary

Reproduce the actual event sequence when debugging. Inspect the owning state and effects, not only the final DOM. Verify the corrected transition plus the relevant neighboring or rejected path. When timing caused the bug, include the applicable overlap, late-result or cancellation case.

Use [review evidence](./review.md#choose-evidence-by-risk) for user-visible checks and the general model's tests for consequential invariants. A list of named states, successful API response or type check alone does not demonstrate a coherent user journey.
