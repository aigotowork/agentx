---
schema: task
status: completed
description: Align the migrated visual Agent space with native AgentX concepts before UI/API adaptation.
links: [../agent-space/task.md, ../../.docs/roles-workspaces.md, ../../.docs/model/README.md]
---

# Agent space conceptual alignment

## User outcome and selected direction

The user wants the whole transplanted Bots conceptual system aligned to AgentX, removing inappropriate old concepts and explaining relationships. User analogy: one AgentX is a large office; directory teams are small offices; each small office has roles/posts (e.g. Controller/frontend engineer); actual named running instances are Agents. They used “started skill” in this explanation; distinguish Skill from Role and Agent based on the explicit category/instance intent rather than inventing running Skill entities.

Deliver a concrete source-grounded adaptation design and explain how to proceed. This assignment is conceptual/product-domain design, not blanket implementation permission. No API implementation, native changes or running-service mutation. Gabi independently owns API design; consume existing docs/native declarations, do not invent a competing API or delegate to/contact existing unrelated agents. Source migration already completed at `web/agent-space`; preserve its user LAN preview :18967.

## Ownership and justification

A fresh domain-contracts professional owns this bounded semantic design because the question crosses UI analogy, filesystem role resolution, durable participant identity, memberships and existing native contracts; Controller coordination alone does not authorize those specialist judgments. One specialist is sufficient. No delegation. Controller zumi owns Task, root discovery and final user explanation.

Specialist writes: `.docs/agent-space/concept-alignment.md` and `.controller/agent-space-alignment/design-work.md` only. Read current module guide and implementation; inspect the actual model/fixture/scene inputs. Read `.docs/roles-workspaces.md`, accepted role/workspace proposal, relevant `src/agents/workspaces.rs`, config role resolution, launch/identity native declarations, profile/skill ownership, and agent-management API docs where relevant. Exact native declarations own current meaning; avoid loading unrelated implementation history.

## Required result

- Concise domain object/relationship model: AgentX scope, directory workspace/team, role definition/effective role, profile, Skill, named Agent participant/execution, current cwd, runtime/provider session, conversation/message/transcript and status.
- Translate user's large/small office/post/person metaphor to native meaning. Resolve `.agentx` versus `.agentx/roles` discovery and explicit membership correctly; flag display eligibility vs actual current discovery without silently changing core semantics. Handle nested directories, inherited/global roles, global members, zero/multiple instances, exited/resumed/history/namesake identities, and unknown facts. Visual office grouping must not imply controller authority, recursive team membership, runtime existence or task completion.
- Old Bots concepts/fields/features retain/change/remove/defer table, linked to actual transplanted source. No second Team/Bot/clone/plugin authority or roles forced onto ordinary agents.
- A concrete UI information/navigation/interaction plan using the existing scene + chat: large-office directory overview, small-office occupants, separate available posts/skills, opening an actual Agent, empty/unknown data, global area and historical views. Keep information compact and preserve selection/drafts/camera behavior. Do not redesign art or implement.
- Finite later adaptation slices and factual requirements to consume from gabi's API (conceptual supply needs and authoritative references, no competing method/schema definitions). Distinguish product decisions, current support, UI-local projection and open supply gaps.
- Explain a representative directory with inherited/global role and two named instances of one role; distinguish their membership from cwd, provider session/history and Skill use.
- Record meaningful assumptions/friction in design-work.md and disposition them. No permission gate for routine decisions; only mark material unavailable facts honestly.

## Completion

A fresh implementer can read the concept design and determine which current UI fields/structures need changing, which native source owns semantics, which proposed visual grouping is presentational, and which API-dependent work remains. Return concise recommendation and exact output links to zumi. No tests/build are required for design-only files.

## Binding

Domain professional: kiri, role `domain-contracts`, profile `codex-gpt-6-1-sol-high`, runtime ready in batch `c7aa1701`; assignment accepted and design delivered (message #20940). Owns only concept-alignment.md and design-work.md, no delegation, API/backend/UI source writes or live resources. The bounded design assignment is closed; no source/API implementation was dispatched.

## Accepted design contribution

The source-grounded [concept alignment](../../.docs/agent-space/concept-alignment.md) and [working record](design-work.md) fulfill this design Task. They specify native concept relationships, old-field/feature dispositions, compact scene/chat adaptation, concrete edge cases and finite later slices. The recommended product/UI projection remains proposed; this closure accepts a usable design contribution, not implementation or live behavior. The existing offline preview remains unchanged.

Material supply gaps: current management projections lack adopted workspace membership/Role, office discovery/default endpoint information, and Role/independent-workspace launch selectors. Do not infer them from cwd or create a competing API; the design supplies conceptual requirements for the separate API owner. A Skill catalog/usage model remains optional and unavailable across all Providers. The recommended next implementation is the bounded offline semantic alignment slice; real read/action binding follows authoritative supply. No test/build is required for the two design documents.
