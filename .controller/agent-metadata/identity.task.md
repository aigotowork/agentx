---
schema: task
status: completed
description: Simplify native participant identity, persistence and message addressing
---

Read [entry](AGENTS.md) and [overall outcome](cleanup.task.md). You own `src/agents/**`, `src/db/**`, and `src/messaging/**`, including their tests and module docs. Remove obsolete Agent metadata completely: fields, SQL, APIs, display prefixes, alias resolution, group routing, helpers and feature tests. Preserve bare native names, remote addressing, exact participant/incarnation, mailbox and generation semantics. Do not implement labels. Fresh schema contains only current native fields; add a safe existing-schema upgrade that drops the obsolete column while preserving identities/rows. Coordinate shared constants or external callers by reporting required edits to maho. Do not edit files outside your ownership. Do not run Cargo/build checks while concurrent owners edit shared inputs; maho coordinates final independent validation. Work directly in the shared checkout, no commits or live service changes. Report completion through AgentX with affected contract signatures and remaining integration needs. Prefer source removal over compatibility aliases.

## Source handoff

bali source release 39508; native storage, routing, archive projection and caller contracts are available for integration. Final executed guarantees belong to [verification](verification.md); source handoff does not imply live adoption.
