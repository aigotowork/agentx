---
scope: AgentX domain SDK contract projection
status: active
owner: domain-contracts
related: ["../../../.controller/typescript-domain-sdk/tasks/canvas-completeness.task.md", "../../model/typescript-domain-sdk.md", "../../../packages/agentx-client/src/domain/contracts.ts", "./schema.d.ts"]
last_updated: 2026-11-21
---

# Canonical contract graph

[contract-model.json](contract-model.json) projects the implemented public [domain contracts](../../../packages/agentx-client/src/domain/contracts.ts), their method ports, data dependencies, observable exceptions and native receipt branches. It is a static visualization fixture. Native Rust, emitted Management declarations and SDK implementations own actual behavior. The earlier relationship Atlas is not an implementation dependency.

| Artifact | Responsibility |
| --- | --- |
| [portable schema](../../../tools/contract-canvas/schema.d.ts) | Stable renderer boundary and relationship vocabulary |
| [layers.js](../../../tools/contract-canvas/layers.js) | Pure layer/anchor lookup and cross-layer edge projection, embedded in generated graph.js |
| [contract-model.json](contract-model.json) | Canonical curated facts, identities, field anchors, method signatures, support status and source evidence |
| [graph.js](graph.js) | Deterministic classic-script projection of the model, [AgentX view](../view.json) and layer helpers for `file://`; exposes `CONTRACT_MODEL`, `CONTRACT_VIEW` and `CONTRACT_LAYERS` |
| [fixture.json](fixture.json) | Required type identities, decisive directed navigation routes and forbidden stopped-Agent ports |
| [check.mjs](check.mjs) | Reference/source-anchor integrity, dependency-edge coverage, ownership, scenario and semantic boundary checks; shared-model validation and static artifact emission/equality |

## Consumer boundary

See the tool’s [model authoring guide](../../../tools/contract-canvas/docs/model.md) for portable types, field anchors, edges and provenance; its [maintenance guide](../../../tools/contract-canvas/docs/maintenance.md) documents the generated browser API. AgentX uses the same contract and adds the conventions below.

Node IDs use public type names (`MemberPage`, `OperationReceipt`) or stable specialized names (`PageRequest<MemberCursor>`, `PreparedOperation<'launch'>`). Method node IDs use `Owner.method`. Overloads share the original method anchor and display the exact declaration signatures, with distinct clickable input and output types. `SelectAgentXScope.selectAgentXScope` represents the public function as a port. Selected underlying HTTP adapter ports keep their `Management.*` IDs, and generated native DTOs retain their existing names. Generic specializations explicitly link to their base shapes; none adds a public SDK export.

The AgentX checker resolves evidence paths relative to the repository root and verifies the recorded source lines. A rejected SDK observation is distinct from a native `Failed` or `Unknown` receipt value. The model captures both through their actual contract identities.

## AgentX layer classifications

The [layer concept and proposed scenario](../layers.md) explain classification and invariants; [Layer Task](../../../.controller/object-atlas-v2/tasks/layer-model.task.md) owns current evidence. Four ordered layers (`application`, `sdk`, `transport`, `platform`) are declaration ownership boundaries independent of node.domain and renderer visibility. Every node has `layerId` and `provenance`; fields inherit those from their owning node and methods match their owner. `application` is a design-only office proposal. SDK/native support states remain independent of provenance; a proposed edge into an implemented SDK port remains proposed.

Source-backed CurrentAgent/StoppedAgent interfaces extend AgentReader<R>; they do not inherit runtime classes. Proposed CurrentActor extends proposed AgentActor; AgentActor implements proposed Actor while holding the SDK Agent union. No edge makes Agent a class base.

The original Office/Actor proposal has seven objects: Office, Actor, AgentActor, CurrentActor, ActorPage, ActorEntry and OfficeRole. Its six original ports are Office.members/roles, Actor.snapshot, AgentActor.snapshot and CurrentActor.prepareMessage/send. CurrentActor inherits snapshot/address through the proposed base, so no duplicate CurrentActor.snapshot declaration is authored. Office.members delegates to exactly one bounded SDK membership page; original order/count, snapshots, cursors and coverage remain observable. OfficeRole wraps the current catalog value, not an actor's adopted role. Downstream SDK values remain navigable and preserve native Failed/Unknown receipts, response-loss recovery and stopped-Agent restrictions.


`office-conversation-draft` adds the [five-step Space contract walkthrough](../layers.md#proposed-space-session-exact-actor-conversation-draft). `Office.openSession` composes a matching bounded ActorPage and ConversationSource into OfficeSession. The session selects exact actor keys and opens a fixed ActorConversation. That conversation reads an application ConversationRead, edits a local addressed MessageDraft, and assesses advisory SendAvailability separately. Close and exact reopen retain the slot. Only app declarations are foregrounded in the walkthrough; no send operation, navigation effect or source invocation is performed by the Canvas.

In the proposal, application-owned receivers and state are available through implementation disclosure while the workflow foregrounds public application values. Proposal anchors point to the design; the cited Space source shows existing responsibilities, not the proposed class hierarchy. Proposed nodes use `sdk: deferred` and `native: not-applicable`.

## Decisive domain paths

| Caller question | Directed navigation |
| --- | --- |
| What does the Workspace page include? | `Workspace.memberPage` → `NativeMemberPage.coverage` → `NativeMemberCoverage.next` → `NativeMemberCursor`; the older `Workspace.members` uses `MemberCursor` and reports the global scan. |
| What is adopted versus observed? | `AgentSnapshot.membership` → adopted / legacy / unavailable; `AdoptedRole` records launch time and `RoleDefinition` records the current catalog. |
| Which conversation record am I reading? | `CurrentAgent.messages` → `MessagePage.checkpoint` for AgentX events; `requestTranscript` → typed receipt → `Transcript` for a bounded Provider observation with no continuation. Both are also available on an exact stopped handle. |
| Can I navigate a recorded launch relation? | `launchRelations` → `RelatedConversation` → `creation` is the historical identity; optional `agent` → `AgentSnapshot.address` is the navigable current or stopped observation. |
| How is the original operation recovered? | `AgentXScope.operation` takes the retained `PreparedOperation<K>` or legacy `PreparedMessage` for typed observe/wait; `OperationAddress` still binds raw `OperationHandle`. Successful results narrow by command; Failed and Unknown remain receipt values. |
| How do I find a newly launched conversation? | `launch`, `resume` or `fork` → typed receipt → `LaunchResult.agentAddresses` → exact `AgentAddress<CurrentReference>`. Null means the receipt supplied no exact references. |
| What is scoped to the listener? | Follow `AgentXScope` to `attachments` for binary registration/download, `inbox` for explicit handling, `directory` for preview and exact send results, or `officeActivity` for an acceptance watermark. |

Anonymous shapes such as `MemberCoverage`, `NativeMemberCoverage` and `AttachmentsUploadInput` are marked `sdk: projection`. `AgentReader<R>` is a private structural interface shared by the public current/stopped interfaces; `Agent` is their union. Composition edges express handle/value composition without implying lifecycle containment in the daemon.

## Source and support policy

Every node and port has separate SDK/native support fields. SDK states are `implemented`, `projection`, `external` and `deferred`; native states are `implemented`, `mapped`, `not-applicable` and `deferred`. `mapped` means SDK shape or capability assembled over existing native contracts, rather than a same-named native object. `external` covers JavaScript/Web platform inputs. These statuses describe source support, not registry publication, browser adoption, installed binaries or verified live effects.

`Workspace.members` reads one global Agent page and filters it in the SDK; an empty page can continue. `Workspace.memberPage` filters in native before applying the limit. It requires the pinned `supportsWorkspaceMembers` flag and binds continuation to the workspace and `currentOnly`. An older listener lacking the flag can still use the legacy method explicitly.

Native message acceptance is separate from delivery, reading, model execution and reply. The original `PreparedMessage` and `OperationAddress` survive response loss. `OutcomeUnknown` retains the original native submit input. Local observation cancellation/deadline throws `ObservationStopped` and does not cancel native work. `wait` returns native terminal Failed/Unknown receipts; it does not apply `waitForSent`'s additional narrowing/refusal behavior. Native `OperationStatus.Unknown` is separate from local `OutcomeUnknown`, even though both require preserving identity for reconciliation.

## Maintain and verify

From the repository root, run `node .docs/object-atlas-v2/model/check.mjs`. Add `--write-wrapper` after intentional source or view edits to regenerate and verify the static entry and data script together. The checker verifies the graph schema, links, source lines, directed navigation, selected compatibility rules and artifact equality. Review a shifted source anchor against its declaration before reanchoring it.

The [Canvas Task](../../../.controller/typescript-domain-sdk/tasks/canvas-completeness.task.md) records current static evidence. The [extraction Task](../../../.controller/contract-canvas-tool/tasks/tool.task.md) records earlier renderer evidence. SDK and native verification stay with the [current Delivery](../../../.controller/typescript-domain-sdk/completeness.delivery.md).
