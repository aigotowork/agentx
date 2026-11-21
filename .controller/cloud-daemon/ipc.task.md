---
schema: task
status: active
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"femi","incarnation_bits":4745316826458155090}}]
---
Read [delivery contract](AGENTS.md).

Own authenticated local IPC and cloud-mode native command routing, including local launch/send/list/listen, hosted agent receive and exact process ownership. Preserve direct legacy installations. Coordinate host owner to share headless launch and exact-stop identity. No token in agent tool context and no unrestricted proxy. Own cloud/client, native IPC and command dispatch as needed. Config as described in host.task.md.

### Owned design

- Configured CLI selects the native scope by path and acquires the ordinary daemon when originating outside an Agent. An already hosted Agent only connects to its own private IPC endpoint. The independent, bounded Linux lane uses native scope lease, frame and kernel peer primitives; it never blocks Application dispatch on HTTPS.
- The IPC contract contains finite messaging and directory operations. The daemon owns the bearer and builds every upstream request. Same-UID callers may query the directory, while actor operations additionally resolve the exact process record, local database generation, cloud generation and installation. A descendant of a hosted Agent cannot use `--iam` to select another Agent.
- Local and remote launches reuse the private Codex supervisor, which records exact process identity before registration and publishes readiness only after Provider startup. Remote journal recovery uses the preselected incarnation. Stop uses pidfd, verifies the immutable process incarnation and targets the exact supervisor. Existing direct hosts and direct CLI retain their original transport.
- Verification cases: reject cross-actor and stale identity; ensure host and tool contexts contain no bearer; distinguish an unready process from ready recovery evidence; protect a replacement from stale stop. The independent acceptance Task owns actual multi-host results.

### Source handoff

| Entry | Role |
| --- | --- |
| [Native cloud dispatch](../../src/cloud/client.rs) | Selects an enrolled scope without reading its token. An exact hosted context takes precedence over environment-selected scopes. `daemon` and operator help retain their native entries. |
| [Messaging vocabulary](../../src/cloud/client/port.rs) and [IPC server](../../src/cloud/client/ipc.rs) | Fixed messaging and directory operations, bounded native frames, kernel peer identity, generation checks and no caller-supplied upstream URL or control endpoint. The only inherited Agent context for this path contains the canonical local scope and incarnation. The private native files and socket share the account's operating-system trust boundary. |
| [Supervisor and shared launch](../../src/cloud/client/host.rs) | CLI and management compose local Codex role/profile and runtime the same way; the management caller passes an exact exported workspace and freezes both daemon and cloud bindings for initial registration. The existing mailbox journal confirms native user-input evidence before completing automatic delivery. |
| [Linux process ownership](../../src/cloud/client/ownership.rs) | Atomic supervisor records provide exact process and readiness facts. A recorded attempt alone is never ready. Exact stop uses a pidfd; restart recovery requires positive exit evidence and does not signal a recovered or replaced process. |

Configured hosting and exact stop currently use Linux IPC and pidfds. No Provider sandbox configuration changes were introduced by this IPC slice. Existing direct-context workers use their existing receiver, including after a client binary change. Installed services and existing Agent processes are outside this delivery's source checks.

### Verification

`cargo test --bin agentx --locked cloud::client::` passed all three focused Linux cases: [finite IPC surface, generation fencing and hosted ancestry](../../src/cloud/client/ipc.rs), plus [pidfd stop, stale target rejection and exit evidence](../../src/cloud/client/ownership.rs). Scoped formatting and Git whitespace checks passed.

The independent first preview established native auto-start, a zero-Agent lease, and replay of a submitted control operation whose response was lost. It used an earlier build whose same-process IPC guard incorrectly rejected the daemon's own discovery before remote launch. The current source permits that authenticated discovery while retaining the actor checks. [Acceptance](acceptance.task.md) owns testing the fresh frozen candidate with the real Provider and physical topology.

Candidate 2 launched both exact Codex hosts. Its [real tool observation](../../.tmp/cloud-daemon/acceptance-local-candidate-2/model-observations.json) proved direct HTTPS denial and absent cloud credential environment, but Linux Codex 0.171.0 also denied the filesystem Unix socket before IPC admission. The [standalone probe](../../.tmp/cloud-daemon/codex-unix-allow-probe/result.json) found the same distinction: without a sandbox the selected Unix socket, a second Unix socket, and the owned TCP listener were reachable; network-off denied Unix `connect` and TCP `socket`; the [documented exact Unix socket allowlist](https://developers.openai.com/codex/permissions/#unix-sockets) in managed proxy mode still denied both Unix `socket` calls and did not reach the host TCP listener. The [pinned Linux upstream test](https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/linux-sandbox/tests/suite/managed_proxy_unix_sockets_tests.rs) explicitly expects a per-path grant to stay restricted. The frozen source remains unchanged; this policy cannot establish the requested sandboxed CLI messaging, and separate physical launch/stop evidence does not establish it.
