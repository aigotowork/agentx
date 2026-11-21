---
schema: document
status: active
owner: agentx
last_updated: 2026-11-08
description: Durable engineering rules for domain-owned native contracts and client/daemon composition.
links: [./README.md, ./contracts.md, ../../.inbox/rust-contract-design/domain/principles.md]
---

# Engineering principles

These project rules apply the [Rust-native contract method](../../.inbox/rust-contract-design/domain/principles.md) to AgentX. They express durable decisions, not temporary worker assignments or test results.

| Situation | Decision and implementation route |
| --- | --- |
| Add a capability or change an invariant | Identify domain/state/transaction owner, then change its native Rust declaration and implementation. Use the [model](../model/README.md); do not invent an IPC operation first. |
| Offer the same capability through another entry | Bind the same operation types and policy implementation. Derive selected serialization/schemas/clients; never maintain a second editable definition. CLI presentation and external Provider protocols keep their own owners. |
| Compose storage | Only the daemon entry constructs the managed persistence adapter. Clients, hooks, Provider hosts, TUI and relay call capabilities and return unavailable after IPC loss. Tests use explicit disposable storage. |
| Read a Provider-owned database or transcript | Respect that Provider's read-only external adapter contract. Distinguish its scope from AgentX managed storage; the daemon rule does not silently seize external databases. |
| Interpret caller identity/configuration | Carry the caller's explicit context, validate claims against durable bindings/capabilities and observed process evidence. Never use daemon-global environment to choose a client identity or session. |
| Attach or replace a runtime owner | Preserve participant incarnation, execution, binding and controller ownership separately. A timeout or missing observation cannot prove that an old owner exited. |
| Accept a mutation or retry after loss | State the transaction and deduplication promise for that use case. Stable request identity is not a universal exactly-once mechanism. A lost reply is unknown until durable acceptance/evidence resolves it. |
| Deliver mail or activate a Provider | Preserve accepted → claimed/frozen → issuing → observed exact input/output → confirmed cursor distinctions. RPC success, turn completion and business completion are different facts. |
| Perform external process/terminal/network I/O | The real host owns that resource lifetime. Use fenced capabilities and keep I/O outside SQLite write transactions. Codex app-server and Paseo retain separate authorities. |
| Replace or reset storage | Coordinate with daemon dispatch, replace database generation and reject stale callers. Never let a host reopen an old managed file as fallback. |
| Publish lifecycle state after a database commit | The DB generation key is the single persistent truth. Publish its mirror before the next dispatch; invalidate and fail closed on publication failure. |
| Negotiate native wire compatibility | Admit equal protocol epochs across package/build differences. [Protocol compatibility](../model/state-ownership.md#native-build-compatibility) is separate from diagnostic build identity, artifact SHA audit, database schema and OS/domain authority; incompatible wire or operation semantics require an epoch bump. |
| Preserve an existing read-model predicate grammar | Restrict it to its declared projection and read-only semantics. Do not expose arbitrary SQL statements, DB paths, write expressions or generic record patches as service operations. |
| Choose module/package/transport machinery | Start with cohesive Rust modules, native enums/traits, existing serde and OS mechanisms. Add a crate, generator, scheduler or protocol mechanism only for a current verified need. |
| Claim support or performance | Link independent fixed-artifact evidence and its scope. Architecture acceptance, compile success, process tests, fixture behavior and actual Provider evidence are distinct. |
| Decide whether to optimize measured residual cost | Preserve required durable and actual-I/O receipt boundaries; select another correction only from a concrete defect or requirement. [The transport disposition](../model/state-ownership.md#measured-transport-cost-and-accepted-residuals) retains disclosed cold/start costs after correcting the material hash/listener regression; it creates no zero-regression SLA or universal performance claim. |

## Dependency and discovery rules

Domain-owned native types contain no database connection, CLI parser, terminal, HTTP framework or external Provider frame. Domain implementations may depend on private persistence helpers while migration proceeds; actual client/daemon composition and ownership guards establish exclusive storage ownership. Do not describe a method allowlist or new file lock as proof that old direct-SQLite processes are excluded.

Every module entry routes a new owner to its native contract, implementation home, allowed dependencies and verification source. Durable architecture lives here and in the model; assignments/evidence stay in Delivery/Tasks. A new operation must be discoverable without reading chat history. Update affected model/contract/guide links when ownership changes.

Rejected defaults are per-domain microservices, multiple product binaries, machine-global data ownership, SQL/argv RPC, a remote `AgentxDb` mirror, mandatory CQRS/event sourcing, a universal bus, package-per-box layouts, forced PTY delivery and a DI framework. A concrete need can justify a future decision; symmetry cannot.
