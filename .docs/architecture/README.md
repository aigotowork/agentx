---
schema: document
status: active
owner: agentx
last_updated: 2026-11-07
description: Domain and process architecture, native contracts and verified implementation routes.
links: [../model/README.md, ../proposals/domain-driven-agentx-daemon.md, ../../.controller/domain-daemon/delivery.md]
---

# AgentX architecture

AgentX is a modular application with one executable and two composition modes: product client and database-owning daemon. One canonical AgentX data directory selects one daemon. Provider hosts retain their actual terminal/stdio/native-process lifetime. AgentX participants, Provider sessions and external orchestration identities remain separate.

| Question | Authoritative route |
| --- | --- |
| How do actual processes, contained modules, communication and events fit together? | [Interactive runtime atlas](runtime-atlas/README.md): source-grounded modes and event playback, explicitly separate from live monitoring. |
| What belongs to each domain and who owns state? | [Model](../model/README.md) and [state/transaction ownership](../model/state-ownership.md) |
| What rules guide changes? | [Engineering principles](principles.md) |
| Which exact native contracts and implementation homes apply? | [Contracts](contracts.md) and [source release](../../.controller/domain-daemon/contracts-handoff.md) |
| Where can production code open/query SQLite? | [Baseline production inventory](../model/db-access.md) and [final storage permissions/helper paths](../model/storage-boundary.md) |
| How are old direct-SQLite processes retired during upgrade? | [Supported scope maintenance boundary](../model/state-ownership.md#legacy-process-cutover); mixed old/new deployment is not supported |
| How do scope, readiness, IPC and restart work? | [Daemon design](daemon.md), maintained by lifecycle owner |
| How are identity, acceptance, claims and receipts preserved? | [Consistency design](consistency.md), maintained by consistency owner |
| What executes Providers and handles hooks/output? | [Runtime hosts](runtime-hosts.md), maintained by runtime owner |
| How are CLI/TUI/relay and maintenance bound? | [Client design](clients.md), maintained by product client owner |
| Who owns peer replication and remote control effects? | [Relay Task and source release](../../.controller/domain-daemon/tasks/relay.md), [control ownership model](../model/state-ownership.md); CLI relay presentation stays with product clients |
| What actually passed on which platforms? | [Independent acceptance](acceptance.md) and [Delivery](../../.controller/domain-daemon/delivery.md) |

Some linked domain implementation/evidence documents are pending their assigned owners. A selected target or published skeleton is not a runtime support claim. Source declarations are authoritative for operation types; acceptance owns executable evidence.
