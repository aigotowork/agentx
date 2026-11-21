---
schema: document
description: Current collaboration workbench entry connecting scenarios, native contracts, UI design and independently accepted support.
status: active
owner: collaboration-workbench
last_updated: 2026-11-09
---

# Collaboration workbench design

## Purpose and current boundary

The selected target is a complete assignment-to-acceptance experience, not a rearrangement of native diagnostic fields. Start from what a user is trying to accomplish, what must be understood together and the actions that produce a usable result. The user requires professional SVG design before frontend implementation and explicitly permits React when justified. The [Proposal](../proposals/collaboration-workbench.md) owns that choice; the [Delivery](../../.controller/collaboration-workbench/delivery.md) owns execution and outcome acceptance.

AgentX has a native collaboration service and a separate embedded workbench with finite operator writes; the existing diagnostic browser remains read-only. The [completed Delivery](../../.controller/collaboration-workbench/delivery.md#已接受的本地实现结果) accepts the first Linux local assignment-to-acceptance slice: a real two-Agent failure/return/repair/check/explicit-acceptance path with retained history, final narrow/touch geometry, standalone embedded assets and completed artifact/cleanup handoff. Exact supported execution and limits belong to the [canonical evidence](../../.controller/collaboration-workbench/evidence/acceptance.md#final-artifact-evidence-and-cleanup-handoff). SVGs remain illustrative design sources.

For everyday use, start with [startup and operation](frontend.md#startup-and-everyday-use); the repository-local `.bin/agentx` contains the accepted binary and needs no runtime Node or external assets. [Native usage and report semantics](native.md#everyday-entry-and-native-reads) explain exact role-bound reports and operator effects. A running user demonstration and installed/default binaries are separate resources and were not replaced.

The subsequent [completed startup usability follow-up](../../.controller/domain-daemon/startup-usability.md#accepted-local-outcome-and-project-handoff) adds independently verified daemon diagnosis/restart and environment/cwd defaults for the workbench. Its exact candidate is promoted to project `.bin/agentx`, and project daemon/workbench execute matching bytes. In this checkout, use `./.bin/agentx workbench --collaboration` to select the normal environment scope and invoking project; a missing daemon is acquired automatically. [Current evidence](../../.controller/domain-daemon/startup-evidence.md#controller-promotion-and-user-runtime-handoff) distinguishes this later startup image from the retained r8 business/UI acceptance artifact.

## Design responsibilities

| Design question | Canonical owner |
| --- | --- |
| Who is the user, what is the primary working subject, how do assignment/progress/input/results/check/acceptance stay connected, and how does it look? | [Experience design](experience.md) and its [original Task](../../.controller/collaboration-workbench/tasks/experience-design.md); concrete SVG states are linked below. |
| Which existing capabilities can serve that path, what new meanings are needed, which native/browser boundary and frontend/build structure are proportional? | [Capability and engineering design](capabilities.md) and its [original Task](../../.controller/collaboration-workbench/tasks/capability-design.md). |
| What does each resource/action do, how do components own state and how do unavailable, empty, pending and conflicting states appear? | [Resource, operation and component details](interface-details.md), released by the same original experience Task with thirteen rendered SVG scenes. |
| What exists today, and which product identities and responsibilities must remain intact? | [Current domain model](../model/README.md), owning native declarations and [browser contract](../observability/browser-api.md). |
| What is active and who may write/execute? | [Team entry](../../.controller/collaboration-workbench/AGENTS.md) and [actual bindings](../../.controller/collaboration-workbench/controller.md). |

## Current implementation sources

The [frontend source guide](frontend.md) and [Web module entry](../../web/workbench/AGENTS.md) describe the component/Block/Page implementation and fresh dual-app embedding seam. Independent fixed-candidate checks and final r8 browser/narrow/touch/embedded-artifact observations passed within the [recorded scope](../../.controller/collaboration-workbench/evidence/acceptance.md#final-artifact-evidence-and-cleanup-handoff). Source Tasks and independent acceptance are completed, all authored writes/resources are released, and [controller closure](../../.controller/collaboration-workbench/controller.md#formal-delivery-closure) records the final decision and bindings.

The [native declarations and report grammar](native.md) and [closed browser contract](browser-api.md) replace callable design sketches. Native aggregate behavior, daemon persistence, finite HTTP and report CLI have passed scoped cases and the actual two-role fixture; the storage-policy/model receiving update is complete. [Independent acceptance evidence](../../.controller/collaboration-workbench/evidence/acceptance.md) owns exact checks, failed attempts, actual support and cleanup; [outcome fixtures](../../tests/collaboration_workbench/README.md) bind the real scenario. The demonstrated Provider lane uses the exact staged executable and isolated full-access roles; it does not establish universal Provider/hook/platform compatibility.

## Actual visual offering

**Current offering:** the user broadly selected the resource-workspace direction after rejecting the first sparse, explanation-heavy design. The original experience Task has now released its resource, operation, component, state and layout details under the [shared design standard](../../.agent-guides/scenarios/interface-design.md). All thirteen final SVGs have source-matched discussion previews; [the receiving record](../../.controller/collaboration-workbench/controller.md#final-detail-receiving-outcome) owns observations and limits. The [before/after inventory](experience.md#历史修订同视口同数据的前后比较) remains historical. These static designs are not implemented support or interaction acceptance.

The four drawings below establish the overall composition; [the detailed offering](interface-details.md) specifies their controls, additional resource views and states. All current sources and previews show five persisted work records and one independent page-local draft. Incomplete creation has no native work ID and is excluded from server/source/resource counts. The [original experience Task](../../.controller/collaboration-workbench/tasks/experience-design.md#current-detail-deliverable) owns this completed design slice, within the same Delivery.

The revised experience presents source groups and aligned work rows with implementer, checker, result, check and next-action columns. Source/Agent resources stay on the left; the selected task's question, assignment or evidence opens locally below the work matrix. These editable source drawings illustrate the same CSV-export scenario alongside other representative work; they are proposed design data, not screenshots of implemented support. Source grouping projects saved references and does not introduce a new planning storage domain.

| Scenario | Editable SVG |
| --- | --- |
| Respond to the implementer's current question | [Main workbench](visuals/01-workbench.svg) |
| Save work and explicitly dispatch to selected participants | [Local assignment editor](visuals/02-assignment.svg) |
| Inspect exact result/check evidence and explicitly accept | [Result and acceptance](visuals/03-result-acceptance.svg) |
| Continue the same question on a narrow screen | [Narrow workbench](visuals/04-narrow.svg) |

The released implementation follows the capability design's bounded daemon-owned work record and React/TypeScript/Vite workbench, retaining the existing diagnostic app and fresh Rust-embedded assets. Actual native declarations and scoped installed dependencies are now available; executable/browser/Provider/standalone support remains independently attributed to acceptance evidence. Do not repeatedly request approval of the already selected overall direction.

## Resource, operation and component details

| Area | Editable SVGs |
| --- | --- |
| Resource views and relationships | [05 Agents](visuals/05-agent-resources.svg), [09 results/artifacts](visuals/09-result-resources.svg), [11 checks/decisions](visuals/11-check-resources.svg) |
| Creation, editing and role selection | [06 draft/validation](visuals/06-create-validation.svg), [12 edit/conflict/discard](visuals/12-edit-conflict.svg), [13 narrow picker](visuals/13-narrow-picker.svg) |
| Messages and evidence recovery | [07 recipients/receipts/unknown outcome](visuals/07-messages-recovery.svg), [08 failed check/history/change reason](visuals/08-check-history.svg) |
| Shared visual and interaction vocabulary | [10 component/state reference sheet](visuals/10-component-states.svg); this larger author reference is not a production screen. |

[The canonical detail source](interface-details.md#组合图覆盖与交接) links every editable SVG and the native/Web/acceptance Tasks. It owns field/action/return/availability design, semantic composition and tokens, keyboard/focus/touch, responsive/scroll rules and critical scenario obligations. Native declarations and current frontend docs own the implemented refinement; independent evidence owns observed support.

SVG data remains illustrative; actual product results and screenshots are separately attributed in the evidence. Retain the running LAN demonstration; it is neither a fixture to overwrite nor a source of automatic write authority.

