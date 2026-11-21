---
schema: document
owner: architecture
status: active
last_updated: 2026-11-07
description: Current AgentX observation contracts, source releases and independent acceptance routes.
links: [../proposals/agentx-observability.md, ../../.controller/observability/delivery.md, ./.xmeta.yaml]
---

# AgentX observation foundation

This foundation explains runtime state from daemon and originating-client viewpoints through native inspection, CLI and a local browser console. The [selected Proposal](../proposals/agentx-observability.md) owns the change; the [completed design Delivery](../../.controller/observability/delivery.md) records design acceptance and the [accepted implementation outcome](../../.controller/observability/implementation.md#accepted-local-implementation-outcome) records selected local support. [Native evidence](../../.controller/observability/implementation-evidence/native.md) and [consumer evidence](../../.controller/observability/implementation-evidence/consumer.md#final-independent-consumer-handoff) own executed results and platform/coverage limits. The [completed daemon outcome](../../.controller/domain-daemon/delivery.md#accepted-local-outcome) remains the preserved prior baseline.

The user selected [direct LAN access and standalone resources](../../.controller/observability/lan-usability.md) as a follow-up to the accepted local implementation. Its original Task records current build/startup evidence. The new normal launch is `.bin/agentx observe-web --scope /absolute/existing/scope`; add `--host 0.0.0.0 --port 18731` for LAN access. Page resources are embedded in the executable; `--assets` is an optional development override. The page opens directly without credentials. The console passively reads an existing daemon; ordinary AgentX operations start the daemon and supply real activity. See the [browser contract](browser-api.md#launch-and-asset-handoff) and [Web maintenance](../../web/observability/AGENTS.md).

| Question | Authoritative route |
| --- | --- |
| What facts and identities exist, and who may change them? | [Context/domain model](model.md) and existing [state ownership](../model/state-ownership.md) |
| What exact shared refs, clocks, evidence and compatibility rules compose the domains? | [Shared contracts and offering catalog](contracts.md) |
| How are daemon, client attempts, connections, hosts and request state inspected under congestion? | [Runtime/client design](runtime.md) |
| How do logs, metrics and traces fit together across AgentX processes? | [Telemetry coverage and optimization](telemetry-coverage.md) |
| What does a slow Codex callback wait for, and how should the path change? | [Source audit](codex-hook-latency.md) and [optimization proposal](../proposals/codex-hook-critical-path.md) |
| What can the daemon move off the reply path after commit? | [Observer wake and subscription selection proposal](../proposals/daemon-postcommit-performance.md) |
| What was captured, selected, validated and resolved for this invocation? | [Configuration/context design](configuration.md) |
| How do events correlate, replay within bounds, expose gaps and stop at reset? | [Events/retention design](events.md) |
| Where will native contracts/implementations live and how is context maintained? | [Engineering map](engineering.md) |
| Can a real diagnosis consume these contracts and what remains unverified? | [Scenarios](scenarios.md), [consumer boundary](consumer-boundary.md) and [design acceptance](acceptance.md) |
| How is the released diagnostic CLI used and maintained? | [CLI usage](cli.md), [source guide](../../src/cli/commands/observe/AGENTS.md) and [process scenarios](../../src/cli/commands/observe/cases.md); execution evidence remains separate |
| What can the local browser adapter read, and how are authentication and assets maintained? | [Browser API contract](browser-api.md), [native adapter guide](../../src/observability/web/AGENTS.md) and [browser bindings guide](../../web/observability/api/AGENTS.md); [consumer evidence](../../.controller/observability/implementation-evidence/consumer.md) owns executable support |
| What was delivered, verified and left outside scope? | [Implementation plan](implementation-plan.md) and [accepted implementation entry](../../.controller/observability/implementation.md#accepted-local-implementation-outcome) |

The [artifact catalog](.xmeta.yaml) lists actual local resources. Exact domain types/operations stay in the owning offering; current native declarations stay in source. A successful diagnostic query does not grant reset/process/Provider authority, complete global visibility or a replay guarantee after an unknown business outcome.
