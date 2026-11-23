# Public HTTP consumers

Read [full consumer Task](../../.controller/contract-service-projections/tasks/full-consumers.task.md), [accepted contract](../../.docs/api-panorama/service-contract-design.md) and [controller](../../.controller/contract-service-projections/controller.md). This standalone package owns browser-safe SDK `@agentx/client` and separate Node bin `agentx-http`; it never changes native `agentx` grammar. Root npm scripts/lock remain separate. [Pilot Task](../../.controller/contract-service-projections/tasks/pilot-consumers.task.md) owns retained bounded evidence, not full support.

| Owner | Responsibility |
| --- | --- |
| `src/index.ts` | Browser-safe public package entry and independent endpoint composition |
| `src/domain/` | [Scoped object model](../../.docs/model/typescript-domain-sdk.md): immutable addresses, native and projected Workspace member pages, exact Agent history/provenance/lifecycle, scope resources, and typed original receipt checks over Management |
| `scripts/generate.mjs`, `src/generated/{management,workbench,observation}` | Actual emitted OpenAPI → fixed generator → wire SDK/types/schema map; never hand-author generated DTOs |
| `src/schema-codec.ts`, `src/codec.ts` | Schema-directed ordinary management integers versus marker-coded Workbench/Observation; exact native widths and strings |
| `src/client.ts`, `src/management.ts` | Management domain operations, original operation receipt identity and local observation |
| `src/workbench.ts`, `src/observation.ts` | Closed native variant methods; operator-only writes and passive evidence reads |
| `src/transport.ts` | Finite listener admission/transport errors, detached retained mutation intent |
| `src/cli*.ts` | Node-only grammar/config/files/stdin/pending request storage and actual SDK binding |
| `src/cloud-admin.ts`, `src/cloud-scope.ts`, `src/cloud-transport.ts`, `src/cli-cloud-admin.ts`, `src/generated/cloud-*` | [Cloud scope](../../.controller/cloud-scope/AGENTS.md): separate bearer clients, native emitted admin/communication projections and private exact admin replay |
| `tests` | Focused consumer/codec/recovery cases; finite injections are distinct from native integration |

Three base URLs remain independent. Native Services own state/authorization/effects; a typed public input never grants actor authority. Inspector output is not a management Operation. Workbench `Rejected` and Observation `LocalFailure` remain native values. Cancellation/close stops local observation only. Preserve original generation, IDs, exact references, revisions and full write input through unknown results; never refresh/retry/rebase automatically. Binary attachments stay raw bytes. Workbench/Observation CLI file inputs/stdout use actual integer markers; SDK memory uses derived bigint types.

One cohesive batch then concentrated verification/integration. Ruzu owns the retained external consumer workspace and combined candidate/Cargo/package lane; coordinate actual source release/repairs directly. No new environment or package generation for ordinary handoffs, no redundant passing checks, no installed/live mutation. Source, finite tests, actual export, installed public package and real native/Provider evidence stay distinct in their owning Tasks.

For cloud daemon host capabilities, exact launch/stop and operation receipts, follow the [wire](../../.controller/cloud-daemon/wire.md) and [consumer task](../../.controller/cloud-daemon/consumer.task.md). The Admin SDK recovers a receipt against the original identity and payload; a successful submit receipt can still be queued. Node pending files remain private before dispatch. Host worker lease/poll/report are present in the generated scope projection and owned by native uplink, separate from the public scope communication convenience methods.

For operator conversation use `admin.listMessages(scopeId, query)` and `admin.sendHumanMessage(scopeId, input)`. See the [shared Space wire contract](../../.controller/cloud-space/wire.md#operator-conversation-for-the-shared-space) for exact addressing and replay. Member reads continue through `scope.listMessages(query)`.
