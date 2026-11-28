# Cloud-connected native scope daemon
For the current Namespace/Workspace cutover and public API, use [the active Namespace delivery](../cloud-namespace/AGENTS.md). This directory records the earlier cloud Scope generation.

Use the [daemon control wire](wire.md) for the Admin and Scope routes, operation state, lease fencing and native OpenAPI source. Follow the [API Task](api.task.md) for implementation evidence and the [acceptance Task](acceptance.task.md) for end-to-end execution.

Status: native control delivery verified on three Linux hosts. Codex 0.171 restricted tool IPC is an outstanding Provider limitation; see the [delivery outcome](delivery.md). Follow the [existing cloud contracts](../cloud-scope/AGENTS.md). The native scope daemon owns the outbound HTTPS connection. Admin requests select an exact installation, an exported workspace, a supported role, and a single typed launch or exact-instance stop. Local tools use authenticated local IPC for scope messaging. No generic tunnel, arbitrary command endpoint or change to the Provider sandbox. Old direct-connected hosts retain their existing receiver until exit. Keep installed services and unrelated worktree edits intact.

The private, owner-only scope file cloud.toml contains url, token, optional host, and [workspaces] alias-to-absolute-path entries. An absent file selects ordinary local behavior. The native service reads configuration and provides diagnostics without tokens. Management availability requires the daemon's own live lease. Local callbacks must bind exact scope generation, installation and incarnation. Validate readiness, uncertain execution, stale identity, retries and disconnects.

| Source | Ownership |
| --- | --- |
| [Control wire](wire.md), [API task](api.task.md) | Cloud capability and operation types, lease transitions, generation checks and durable observations |
| [Host task](host.task.md) | Native daemon control execution, workspace and role discovery, persisted claim reconciliation |
| [IPC task](ipc.task.md) | Authenticated native dispatch, exact host identity, legacy and connected message ownership |
| [Consumers](consumer.task.md) | Public SDK/CLI, Management Web and native deployment TUI |
| [Acceptance](acceptance.task.md) | Isolated runtime and three-machine evidence |

[Joined delivery record](delivery.md) links the candidate and its verification evidence.

The controller owns scope enrollment, operator diagnostics, integration build, and acceptance handoff. [Host setup and service manager usage](../../.docs/architecture/cloud-daemon.md) and [connection configuration](../../src/cloud/connection.rs) describe local enrollment. The physical acceptance owner operates disposable resources on the previously authorized three machines. Completion requires a frozen binary, observed operation receipts and a cleanup record.
