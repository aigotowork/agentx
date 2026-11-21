---
schema: scenario
description: Design recoverable prototype tutorials with stable targets, observed completion and separate progress and business-effect lifecycles.
tags: [interaction-modeling, tutorials, prototype, recovery]
---

# Build a scenario-driven guided demo

Use when a person needs to learn a workflow by operating a prototype with target highlighting, contextual prompts, dialogs or page navigation. Start from an existing scenario and complete outcome slice. An installed tutorial runtime is optional; its actual API and resources own executable integration. This method neither supplies an SDK nor authorizes external business actions or deployment.

## Bind the lesson to an observable task

Reference original scenario/criterion, flow and model actions. Each step identifies initial object/version/scope, expected action, readiness, visible completion, decisive rejection/conflict/unknown branch, continuation and evidence boundary. Static action membership is not an executable completion rule or proof of workflow order.

Keep presentation definitions separate from host behavior. Presentation can contain stable target IDs, title, concise consequence and placement. Repeated controls need object scope/key as well as a target name. A navigation-only link needs no unrelated business action to satisfy validation.

The host boundary must provide read-only enter/restore, exact identity/scope, observed readiness/completion and meaningful state-change subscription. Require a unique available target or an explicit unavailable/ambiguous state. Bind asynchronous work to the run and step token captured before the operation; a response arriving after exit, restart or step change cannot complete the new step. Unsubscribe and invalidate stale callbacks at the proper lifecycle boundary.

Click, request acceptance, timeout and dialog closure are not completion unless the step promises only that result. For “apply this change,” observe the native result and updated object/version; retain partial, rejected and unknown branches rather than advancing on a timer. A model condition trace cannot provide those observations.

## Separate tutorial recovery from domain recovery

| State | Owner and recovery |
| --- | --- |
| Tutorial progress | Runtime/host: definition version, run/step, checkpoint and permitted resume state |
| Selection/draft/return context | Its UI task owner: declared persistence, lifecycle and invalidation rules |
| Domain object, preview, operation identity and receipt | Original domain/client: re-read and reconcile existing work through its recovery contract |
| Outstanding callback/subscription | Current run/step: cancel observation or ignore stale result; do not infer remote cancellation |

Restore progress and context, then read original domain objects. Never restore by automatically reissuing a mutation. Retain original previews, request identities and receipts; a lost reply may leave an unknown effect even when no completion checkpoint was saved. Declare the atomicity boundary between host effects and progress persistence instead of claiming exactly-once execution across them.

Choose storage for the actual access origin and browser capabilities. Ordinary HTTP LAN origins may lack secure-context APIs. A transactional store or correctly fenced adapter must uphold required multi-tab ownership; localStorage read-then-write alone does not provide exclusion. Handle ownership loss, version mismatch, missing receipts and unavailable storage honestly. Do not invent persistence guarantees to keep a tour moving.

When source/definition changes invalidate progress, explain the mismatch and re-enter from an appropriate read-only checkpoint. Do not transplant progress to a different object, baseline or session. Explicit restart of a lesson resets tutorial state; business reset follows its own authorized contract.

## Verify the useful path and decisive recovery

Use the selected runtime's actual checks and explicit authorized prototype paths. Cover relevant target uniqueness/highlighting, native dialog layering, keyboard/focus, rejected/disabled/ambiguous targets, reload before commit, page navigation, paused exit/resume and stale asynchronous results. Keep highlighted content readable; a mask must not dim or intercept the work the person needs to inspect.

Reference validation can establish existing scenario/flow/action membership only. It cannot prove target uniqueness, permissions, execution order or business completion. Distinguish runtime tests, Mock recovery, actual UI observations and participant research; label unexecuted paths unverified.

Stop when the selected workflow can be followed and recovered under the declared environment, required scoped checks pass and remaining limitations have owners. A single guide cannot establish complete product coverage. The [worked slice](examples/scoped-update.md#recoverable-demo) shows the distinction between a lesson checkpoint and an applied effect.
