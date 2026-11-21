---
schema: document
status: completed
owner: kiri
description: Source-grounded working record for the bounded Agent space concept alignment design.
links: [./task.md, ../../.docs/agent-space/concept-alignment.md]
---

# Concept alignment working record

## Assignment and method

Accepted the [Task](task.md): one semantic/product design joining the user's office analogy, native AgentX facts and the existing scene/chat frontend. Only this record and the linked concept design are owned. No delegation, API definitions, implementation, live configuration, server restart or preview mutation. Zumi owns Task closure and entry-point updates; gabi's separate API ownership is consumed through published/native facts only.

Method: trace one directory with inherited roles and two actual instances through scope → membership → role provenance → exact Agent → messages/model history; compare every current fixture field and scene grouping against its native owner; decide presentation rules and record unavailable supply. Apply domain-contract modeling, caller-supply tracing and shared/local interface guidance. This is a bounded source/design exercise, not a rendered UI acceptance or a test/build task.

## Established findings

- `config/roles.rs` discovers the nearest `.agentx/roles` directory, not any `.agentx` directory. Explicit workspace selection can establish membership without a marker. Inherited/global role origin does not set membership.
- `agents/workspaces.rs` stores optional adopted membership/role snapshots. A directory's team is its actual members; a default message endpoint is a separate native binding.
- `agents/attachments.rs` separates exact participant incarnation, runtime generation, attachment and provider-qualified session. `management/contract.rs` adds exact stopped-event references and dataset generation. Name-only view keys cannot safely carry writes/drafts across reuse.
- The migrated frontend is deliberately offline. Its room source/workspace grouping, role-like display titles, `attention` → unread label, and monolithic synthetic state need semantic adaptation before real data binding.
- Current management HTTP projections expose neither workspace membership nor adopted roles; `directory` is not a substitute. Skill protocol pass-through is not an AgentX Skill registry or usage record.

## Decisions, assumptions and friction

| Finding or question | Disposition |
| --- | --- |
| The user said “started Skill” while explaining category versus instance. | Interpret the explicit intent as a Role used to launch an Agent. Skills remain reusable methods/resources; do not create Skill people. Role, Skill and Profile each retain their native meaning. |
| `.agentx` seems like an intuitive office marker but native discovery uses `.agentx/roles`. | Preserve the native rule. The design separates discovered offices, known explicit memberships and browsed directory contexts. No silent discovery change. |
| Native directory resolution can inspect ancestors/global Roles but cannot enumerate every office in a scope. | G1 is an actual catalog/coverage supply gap. Display the known set and partial coverage rather than invent an exhaustive workspace list or recursive scan. |
| Native current-team listing mixes in global members, while a room implies exact membership. | Group exact members in each room and keep global members in a separate adjacent area. This is a UI projection, not a change to native listing or membership semantics. |
| The management API exposes `directory` but omits adopted workspace/Role/provenance. | G2/G3 block a truthful full office projection. A flat exact-reference Agent reader remains feasible; do not infer office or post from cwd, title or model. |
| Current profile names and model values look interchangeable in the sample. | Separate Role profile reference, adopted launch provenance and observed Provider model. Unknown optional facts stay absent/unknown. |
| A generic attention bit is rendered as human unread. | Do not reuse native Agent mailbox unread as viewer unread. Use separately labeled UI-local observation state if needed; real delivery/read receipts require their owner. |
| Original Bots has configurable teams/persona profiles/library grants and coarse hall states. | The original source and migration inventory explain provenance only. Remove their authority model; reuse the art/layout/reader. Native state and exact Agent identities drive the aligned UI. |
| Current Provider transcript projection is coarser than the transplanted reasoning/tool UI. | Render supplied exchanges honestly. Preserve the renderer for future supported blocks, without synthesizing successful tools or Skill use. |
| Current API launch has no Role or independent workspace selector. | G7 blocks post-based launch and directory-contact actions until the API owner supplies the native meanings. Tool/profile launch is not an equivalent substitute. |
| Role-file changes and resumed/namesake Agents can break name-only views. | Preserve adopted snapshots and exact refs; do not silently transfer drafts, actions or history. New incarnation selection requires an explicit continuation supported by native output. |
| Skill inventory/use and truly current cwd/model are not uniformly supplied. | G5/G8 are optional detail gaps. Mark availability/context honestly; no Skill manager or new telemetry system is part of this task. |
| Product needs compact explanation but the full semantic rationale is substantial. | Put core structure in office/member/post navigation; use contextual detail for origin/provenance. Keep the source-linked rationale in the single concept document, not standing UI prose. |

The representative atlas/web directory and Agent names are authored examples, not observations of the running team. Design uses published declarations/source, not claims about the installed or running binary. No live roster, service or preview operation was needed.

## Final handoff

The [concept design](../../.docs/agent-space/concept-alignment.md) contains the object/relationship model, native discovery and routing rules, identity/history treatment, complete current-field and legacy-feature disposition, scene/chat information and interaction plan, one inherited/global-role scenario, G1–G8 supply requirements and four bounded adaptation slices. It includes finite design cases for the decisive unknown, empty, conflict and history branches.

Final structural check resolved all 87 local Markdown links across both documents, including heading fragments; no missing target or unmatched fragment. Source tracing covered the Task's required concepts and special cases. This check establishes document/source navigability only, not runtime behavior.

Only the two assigned Markdown files were authored. No tests/build/browser acceptance was required or performed; this result is source-grounded design, not implemented UI/API support. The existing LAN preview and all user services remain untouched. Zumi owns acceptance, Task closure, root/module discoverability links and any future implementation dispatch. The next recommendation is the bounded offline semantic alignment, with G1–G3 and G7 resolved through the separately owned API before full real-data office/actions integration. No contact with gabi or unrelated Agents, and no delegation.
