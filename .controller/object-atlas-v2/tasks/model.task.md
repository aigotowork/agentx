---
schema: task
status: completed
description: Define and populate the source-grounded typed contract graph consumed by the Canvas.
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"labu","incarnation_bits":4745315424885665285}}]
---

Read the Delivery, Proposal, TypeScript domain model, contracts, generated Management types and relevant native contract. Own the canonical declarative model and source mapping under .docs/object-atlas-v2/model/; do not implement the visual page or change SDK/native sources.

Define classes, interfaces, enums, value objects, fields, method ports, parameters, returns, errors, outcomes, ownership, composition claims and source evidence. Avoid claiming inheritance where the source uses structural interfaces or composition. A method port is a domain object with stable ID and directionally typed edges. Add relation kinds such as owns, returns, accepts, contains, references, observes, rejects and composes.

## Current model and handoff

The source slice is the complete public domain contract in `packages/agentx-client/src/domain/contracts.ts`, with implementation evidence from the domain adapters, management client, emitted Management declarations and native management contract. The Canvas consumes `.docs/object-atlas-v2/model/contract-model.json`; stable IDs use declaration names and `Owner.method` port names. Inline field IDs are selectable graph anchors; the model contains no layout coordinates. `MemberCoverage` and member row objects are explicitly named visual projections of anonymous source records, not new exported SDK types. Current/stopped readers share `AgentReader` structurally; no class inheritance is claimed. Complete native Workspace membership paging remains deferred, while bounded SDK global-page projection is implemented.

The renderer/API boundary was sent to @nuvi and @kula in AgentX delivery #56052; actual files and the passed integrity result were handed off in #56140. Controller owns independent acceptance.

## Completed structural handoff

- [Canonical model](../../../.docs/object-atlas-v2/model/contract-model.json), [schema](../../../.docs/object-atlas-v2/model/schema.d.ts), [offline script](../../../.docs/object-atlas-v2/model/graph.js), [fixture](../../../.docs/object-atlas-v2/model/fixture.json), [integrity checker](../../../.docs/object-atlas-v2/model/check.mjs) and [source/consumer handoff](../../../.docs/object-atlas-v2/model/README.md) are persisted. The context entry is linked from the Delivery entry.
- `node .docs/object-atlas-v2/model/check.mjs --write-wrapper` passes: 139 nodes, 279 selectable field anchors, 600 directed edges, 121 checked source anchors, four scenarios and nine directed navigation routes. The generated classic script exposes `window.CONTRACT_MODEL` and matches canonical JSON exactly.
- All requested domain types, complete domain object ports, input/output dependencies, local/native/unknown errors, native receipt branches and separate SDK/native support states are explicit. Generated Management dependency closure preserves separate native DTO identities where domain names collide. Native DTOs and SDK/native sources were not changed.
- Native membership paging is a deferred node, not an omitted type or an implied complete implementation. Stopped readers have no preparation/sending ports. Unknown effect and local observation stop remain distinct from native failed receipts.
- Friction resolved: the first authoring pass found a stale assumed store function name (`list_agents`); the actual source owner is `store.rs::list`. The graph stores the actual symbol and line, and the checker now validates all symbol-bearing source anchors. No unresolved modeling choice blocks the renderer.

This completes the requested model/declaration/consumer-data boundary. Browser rendering, UI navigation behavior and independent acceptance remain with the Canvas owner and Controller. No live service or installed package change is made.

Make these first-class and navigable: ScopeAddress, AgentXScope, Workspace, RoleCatalog, Role, MemberCursor, PageRequest, MemberPage, Agent, CurrentAgent, StoppedAgent, AgentSnapshot, Membership, MessageCursor, MessagePage, Message, PreparedMessage, OperationAddress, OperationHandle and OperationReceipt. Explicitly model Workspace.members accepting PageRequest<MemberCursor> and returning MemberPage; MemberPage.members contains Agent and AgentSnapshot; coverage.next returns MemberCursor. Model the send trace and receipt branches similarly. Include absent or deferred native paging as a typed capability status, not as a missing node. Produce a small schema, fixture and reference integrity check. Keep layout separate from facts and hand off a stable model early.
