---
schema: document
status: active
description: Message eligibility, recipient scope and autonomous execution
links: [ ../AGENTS.md, ../roles/controller.md, ../roles/implementation.md ]
---

# Action-based communication

Default to doing the bound task and updating its current state. A message needs a named recipient and a concrete action, decision or newly unblocked dependency. Sending routine progress only to the controller still creates avoidable work; ordinary progress belongs in the task file.

## Send only when needed

| Event | Destination/action |
| --- | --- |
| Implementation, reading, skill selection, recovery, test start/pass, fixture correction, internal plan | Update the bound task only when useful; no HCOM message |
| Actual completed assignment with evidence and cleanup | One message identifying the promised contribution, model/consumer fit and remaining integration so the receiver can accept the result and update the Delivery |
| Callable contract/artifact now unblocks an assigned consumer | Notify that specific waiting owner once; controller only if dispatch/dependency state must change |
| Concrete defect in another owner's released capability | Send that owner one repro, expected behavior and required fix; avoid reporting the same known failure again |
| Blocker outside scope, conflicting ownership, shared-model drift, recurring cross-consumer defect or missing authority | Send the responsible controller/decision owner the affected promise, causal model or uncertainty, consumers and proposed disposition; continue independent work |
| Unsound Task premise, prescribed method or acceptance condition | Expert sends evidence, affected guarantees and a proposed correction to the Controller/real decision owner. Controller revises the context or routes the actual commitment change; neither silent obedience nor a new per-detail approval queue. |
| Fragmented assignment/execution prevents a coherent domain batch | Worker reports the missing system context and proposed boundary to the Controller; Controller addresses the observed pattern with its owner. Use one actionable correction, not per-file permission or routine supervision reports. |
| A real overlapping shared-file write | Coordinate only with the current writer; send release to the next actual waiting writer |
| Explicit status request | Answer the requester briefly, then resume the task |

No general progress feed, group CC, broadcast, success acknowledgement or peer policy forwarding. Keep different dependencies in separate messages to their respective owners. A shared directory does not make every user of it a recipient. Planned/applying/released notices do not all need sending: request only a real overlapping edit and signal only the handoff that releases a waiting writer. Do not create distributed approval queues for independent lines or nonoverlapping leaves.

## Stay inside the assignment

Internal decisions, compatible fixes, assigned exports/registration and explicitly granted adapter/consumer relationships are implemented autonomously. Check existing task authority before requesting permission. A renamed helper or additional internal file serving the assigned capability is not a new functional assignment; actual ownership conflicts or semantic expansion are.

Initial identity/claim is recorded in the task; the controller already receives launch readiness. Send a claim only if explicitly requested or if binding/readiness needs attention. Tool-required skill notices belong in the worker's own commentary, not a duplicate HCOM progress message.

Do not turn unfinished sibling code into repeated peer-check cycles. Continue independent parts until the promised handoff is callable. Report a released-contract defect once; consult the current issue before duplicating a known failure. Route a recurring shared cause through [workflow change handling](workflow-management.md#respond-to-changes-at-their-owner). Detailed per-case results stay in evidence; receiving owners still perform the substantive [outcome acceptance](workflow-management.md#recover-and-close-the-outcome).

## Receiving and controller behavior

An informational message does not assign new work or authorize editing a sibling. Resume your bound task unless it resolves a dependency you already own or supplies a concrete defect within your scope. A successful check notification needs no acknowledgement, source-hash audit or status-document update solely because it arrived. For a promised handoff, apply [outcome acceptance](workflow-management.md#recover-and-close-the-outcome) to the available result. Acknowledge actual requests as the tool/session requires, then provide the needed action/result; never acknowledge an acknowledgement. Do not initiate unsolicited audits or retransmit informs.

The controller applies the same eligibility rule, reads task state for progress, and sends decisions only to owners who must act. A user-directed correction affecting all active workers may be delivered once to each affected worker with no reply/forwarding requested; target their current affected assignments, not completed ones. Worker reuse or retirement follows [assignment closure](../roles/controller.md#close-rotate-and-remember). This exception is not a standing announcement channel.

Before ending a controller turn to await work, reconcile requests addressed to the controller since its recorded request checkpoint. Reply to each actual unresolved request with its reply ID, or retain it as explicitly pending with the next decision action; advance the checkpoint only after accounting for that range. This is a bounded inbox check, not a traffic audit or worker progress poll. Automated delivery and subscriptions do not prove every request was handled. Request-without-reply reminders identify controller work and must be resolved by handling the request.

## Event subscriptions

Before waiting, the Controller applies [ready-work scheduling](../roles/controller.md#schedule-ready-work) to admit any independent Tasks that fit available capacity. Then wait for messages explicitly addressed to the responsible controller. HCOM delivers direct messages automatically; do not add an event subscription that duplicates them. Completion, a real blocker, and a concrete contract decision are sent by the owning agent through the existing communication rules.

Where notification routing is configurable, wake recipients only for actionable requests, dependencies, defects and assigned completion. Routine success or acknowledgement events create no new work. If the runtime delivers them anyway, continue the existing assignment without generating a follow-up activity or reply.

Do not subscribe to routine status changes, idle/listening events, agent launches, global collisions, file edits, command activity, broadcasts, or other workers' conversations. Do not poll list/events/transcripts while waiting. Inspect them only to resolve a specific reported problem or to verify an authorized launch once. An acknowledgement does not require another reply or a user-facing progress update.

## Preserve pending decisions

For an explicit asynchronous request, retain the requested decision, sender/recipient, original request reference, affected scope and next action in the existing receiving work context until resolved. A queued send is not a decision. A follow-up links the original request rather than creating another independently tracked question.

Assigned peers may resolve their existing public-contract relationship directly when authority permits. One dispatch owner still controls each worker; direct collaboration does not authorize retasking a sibling or changing its semantics. Return scope expansion, shared-writer conflict and cross-domain product decisions to their actual owner. Use one completion route, not automatic callback plus duplicate messages and acknowledgements.
