---
schema: task
status: completed
description: Simplify management, Relay and UI consumers
---

Read [entry](AGENTS.md) and [overall outcome](cleanup.task.md). You own `src/management/**`, `src/relay/**`, `src/service/**`, `src/tui/**`, `src/collaboration/**`, `src/daemon/**`, `src/observability/**`, and `web/**`, including tests/module docs in those trees. Completely remove obsolete Agent metadata fields, filters, launch form controls, names/aliases/group addressing and fixtures. Regenerate web native bindings as appropriate after Rust contract changes; preserve unrelated HTML/JSX tags, AEAD authentication tags, generic UI badge components and chart labels. Do not implement labels. Match external native contracts after obsolete fields/arguments are removed. No edits outside ownership, no Cargo/build checks while concurrent owners edit shared inputs, no commits/live service changes. Source/docs transformation tools and formatting are allowed. Report remaining external integration to maho through AgentX. Work directly in shared checkout.

## Source handoff

mike source release 39362; all native consumers and lifecycle history projection are available for integration. Web bindings and fixtures were integrated by maho; bounded Web checks are recorded in verification.md. Final executed guarantees belong to [verification](verification.md); source handoff does not imply live adoption.
