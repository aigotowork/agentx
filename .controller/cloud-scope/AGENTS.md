# Cloud scope delivery
For the current Namespace/Workspace cutover and public API, use [the active Namespace delivery](../cloud-namespace/AGENTS.md). This directory records the earlier cloud Scope generation.


The [delivery record](delivery.md) identifies the verified Linux binary, the public contracts and the cross-network results.

The [three-host LAN experiment](three-host-lan.md) records actual remote deployment, two native clients using real models, remote administration, and recovery after token rotation.

Use the [operator guide](../../.docs/architecture/cloud-scope.md) to deploy one HTTPS service and connect independent local Codex hosts. A deployment bearer manages scopes and metadata. Each scope has one communication bearer and a unique short name per active Agent. Host names and installation IDs distinguish machines within the trusted scope. Local Provider processes, credentials, paths and terminals stay on their originating machines.

| Concern | Owner and source of truth |
| --- | --- |
| Deployment ownership, public routing, admission and static assets | [Deployment entry](../../src/cloud/mod.rs) |
| Admin methods, wire shapes and query coverage | [Native admin projection](../../src/cloud/admin.rs), [generated OpenAPI](../../packages/agentx-client/cloud-admin.openapi.json) |
| Scope provisioning, stable deployment identity, credential rotation and exact mutation replay | [Deployment catalog](../../src/cloud/catalog.rs) |
| Host/participant bindings, message transactions, claims and evidence | [Scope wire and state transitions](communication-wire.md), [communication task](communication.task.md), [generated OpenAPI](../../packages/agentx-client/cloud-scope.openapi.json) |
| Agent launch, local host recovery and CLI dispatch | [Native client task](client.task.md) |
| Generated HTTP clients, credential-safe pending writes and package CLI | [SDK task](sdk.task.md), [package guide](../../packages/agentx-client/README.md) |
| Deployment console and terminal UI | [Management Web task](web.task.md), [TUI task](tui.task.md) |
| Acceptance scenarios and actual test coverage | [Independent acceptance](acceptance.task.md), [runnable matrix](../../tests/cloud_scope/README.md) |

## Design decisions

The root catalog has one owner, while each scope has its own generation and communication database. Creation initializes a provisional directory before publishing the scope and receipt in a single catalog transaction. A failed attempt can reuse that directory. Original requests can recover their token-free receipts after restart or rotation. A token change invalidates outstanding waits independently of the data generation.

Admin pages use short generation-fenced reads for summary counts and bounded host/Agent pages. Unknown coverage survives aggregation into the global overview. Native compile-time DTOs and handlers own public schemas. Regenerate published admin and communication consumers from the corresponding native exporters.

The cloud adapter uses its own participant/incarnation and inbox records because current local participant bindings require same-machine process evidence. Cloud and existing local daemon databases have separate ownership. A probe or failed cloud call must preserve the selected endpoint. Local management and Relay remain under their existing owners.
