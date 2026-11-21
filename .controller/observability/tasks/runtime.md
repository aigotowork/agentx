---
schema: task
status: completed
agent: Runtime and client observability designer
---

# Runtime and client observability designer

## Goal and professional authority

Define the client/server runtime facts and bounded observation lifecycle required to explain reachability, health, current activity and slow or uncertain requests.

Read the [team contract](../AGENTS.md), [Delivery](../delivery.md), [proposal](../../../.docs/proposals/agentx-observability.md) and [actual bindings](../controller.md#runtime-bindings). Apply: Engineering contract design, data/state modeling and source-grounded process/transport lifecycle analysis. Retain selected questions and conclusions in this Task. Current source is evidence; planned contracts must be labeled as design targets.

## Inputs

`src/daemon/{server,wire,transport,scope,platform,windows}.rs`, `src/service/{context,client}.rs`, relevant runtime host/notify declarations, current status and acceptance, plus the architecture owner's shared vocabulary.

## Required result

1. Map current producers/consumers and concrete instrumentation insertion boundaries before selecting storage. Separate client attempted call, transport connection/readiness probe, admitted queued job, dispatched operation, reply delivery and reported external completion.

2. Define data types and identities for daemon runtime snapshots, observed originating processes/logical client hosts, connections, per-attempt request observations, aggregate metrics and health/freshness. Include exact field meaning/units/cardinality and what cannot be observed. Preserve observed PID/incarnation versus caller claims and domain request deduplication semantics.

3. Specify native runtime snapshot/detail/report/query inputs, results and errors plus state-transition tables. Define abandoned connections, expired queued jobs, dispatched timeouts/unknown results, same-PID reuse, daemon restart and generation reset. Active logical hosts cannot be inferred from the instantaneous short-connection count.

4. Show how runtime state remains inspectable during a congested sole Application dispatcher without a second managed database owner or unbounded memory/worker/queue. Define concurrency, collection/update ownership, bounds, pressure, sampling, cleanup and metrics windows/resets.

5. Specify required event-correlation and configuration-context references with the other owners. Client failures before daemon reachability require originating-client evidence; identify capture/report loss behavior and separate local/server latency meaning.

6. Author meaningful normal/failure/pressure/restart/configuration-comparison cases as design fixtures or tables. Name concrete implementation leaves and consumers, but do not edit or execute production code.

## Write and resource boundary

Own `.docs/observability/runtime.md` and this original Task. Coordinate shared-file transfers explicitly. Root navigation, roadmap and proposal/Delivery state remain controller-owned. Read-only current source inspection is authorized. No production source/dependency/package changes, no build/test/runtime probes, no nested agents, and no live/default scope access.

## Handoff and completion

Architecture integrates shared identities; events consumes runtime observation/correlation needs; integration consumes native runtime contracts and diagnostic cases.

Release usable contracts early when they unblock a real consumer. Normal progress stays here; message only actionable contracts, concrete questions/findings or completion through AgentX. The final handoff must identify authoritative artifacts, resolved versus deferred decisions, scenario evidence, downstream obligations and released write/resources. Design completion never claims executable support.

## Working record

Bound agent: `{serverId: srv_yDbm9zuqIMiM, agentId: 4e409b53-d9db-4257-930f-8011ab8b539e}`. Board mirror: `UNS-2`; this file remains the authoritative execution record.

Method: source-grounded contract and state modeling. First map producers/consumers and authority boundaries; then define identities and independent request/result/reply dimensions; then specify bounded in-memory maintenance and native query/error contracts; finally give integration exact scenario fixtures. This design-only assignment creates no source skeleton and executes no build, package, test or runtime probe.

Confirmed initial source facts: `daemon/server.rs` has one Application dispatcher, 16 queued jobs, 32 connection workers and a separate admission loop; Ready is emitted before queueing. `daemon/transport.rs::call` never retries after a possible write. `service/context.rs` separates caller RequestId/invocation claims from OS-observed CallerProcess and independent daemon/database identities. These require server evidence and originating-client evidence to remain separate.

## Released result and method conclusions

Authoritative domain design: [runtime.md](../../../.docs/observability/runtime.md). All new contracts are explicitly planned; their native home is `src/observability/runtime.rs`, with private retention in `registry.rs`. The finite inspection adapter aggregate and passive entry functions belong to `src/daemon/observation.rs`. There is no production source skeleton or executable support claim.

| Method question | Established source basis | Selected result |
| --- | --- | --- |
| Which runtime boundaries can actually be observed? | `server.rs::{accept_connections,connection,enqueue,dispatch_jobs,dispatch,validate,reject_queued}`; `transport.rs::{probe,acquire,handshake,call,request_deadline}` | Separate local attempt, connection/Ready, decoded Call, admission, dequeue/dispatch, Application result, reply-wait abandonment, reply write and client receipt. Native external effects remain with domain receipt owners. |
| Which identity proves what? | `service/context.rs::{RequestId,InvocationContext,CallerContext,CallerProcess,RequestContext}`; `platform.rs::peer`, `windows.rs::peer`, `sys/process.rs::identity` | Invocation/attempt UUIDs are caller correlation; server refs include daemon identity. AcceptedPeer is transport-constructed. Known incarnation can group peers; missing incarnation is connection-local and never merged by PID/None equality. |
| Can ordinary clients support passive diagnosis? | `Client::{connect_existing,with_invocation,call_with_context}`, `client.rs::service_error`, `DaemonScope::{prepare,socket_path}` | Separate read-only existing-scope lookup and inspection adapter; no directory repair, startup or CLI Session. Preserve local stage/Busy/deadline/compatibility/protocol errors before flattening. |
| How can state be read while Application is congested? | `server.rs` 32 workers/16 jobs/one Application; `scope.rs::{ScopeLease,DatabaseGeneration}` and `mod.rs::serve_with_stop` | Same lease, separate scope-keyed inspection endpoint/admission thread, <=2 short workers, 1,000 ms local deadlines, finite capped requests/replies, no waiting queue or DB ownership. Process-only inspection can exist before generation publication. |
| What is a logical host? | `runtime/hosts.rs::{HostRegistration,ProcessRegistration,Request}`; `hosts_store.rs` register/renew/release; `process_host_store.rs` authenticated peer/child validation; `notify/server.rs` connect-and-close wake | Last validated safe host projection, not socket count, full census, host token or universal liveness authority. Domain lifecycle/attachment/receipt policy remains unchanged. |
| How do state and evidence end? | Native generation publication/dispatch fences, no-retry transport and host owner boundaries above; shared era/time contract | Finite active/completed/peer/host/report rows, monotonic TTL and metrics window, checked counters, explicit drops/eviction and no cross-store pin. Reset preserves labeled process chronology but fences dataset refs; restart ends all runtime continuity. |

Consequential alternatives rejected: adding observation reads to the congested business queue cannot explain saturation; a second Application/SQLite owner violates the baseline; durable telemetry/outbox and long-lived inspection subscriptions are not required for the selected diagnosis scenarios; a generic health/delivered boolean would collapse independently observable facts. The selected bounded in-memory registry and short queries meet the design scope without new external dependencies or platform research.

## Consumable sections and resolved consumer findings

- [Identity](../../../.docs/observability/runtime.md#runtime-identity-contract): ClientInvocationId/ClientAttemptId, daemon-scoped ConnectionRef/RequestObservationRef, peer/host/report refs and trusted AcceptedPeer.
- [Passive inspection](../../../.docs/observability/runtime.md#passive-inspection-and-structured-local-failures): ExistingScopeInput/ScopeLookup, structured LocalFailure using ClientStage, InspectionReady and native aggregate/entry functions. Unix endpoint is `{scope_digest}.observe.sock`; the fixed user-wide sibling interpretation was rejected as a cross-scope collision. InspectionResponse::DaemonChanged is route-neutral and precedes leaf dispatch, including Describe and Head(None).
- [Request facts](../../../.docs/observability/runtime.md#request-connection-and-host-facts): effective_deadline is EvidenceValue<ObservationTime>, preserving decode-time rejection without fabricated budget. Execution, reply waiting and reply writing are independent, so timeout followed by failed write does not overwrite either fact. Configuration uses exact ConfigCaptureResult/ConfigStage and bounded multiple load-site refs.
- [Reads](../../../.docs/observability/runtime.md#snapshot-metrics-and-read-contracts) and [origin reports](../../../.docs/observability/runtime.md#originating-attempts-and-report-admission): exact runtime query/result/error branches, active gauges/60-bucket metrics, retained report list/detail, source-time versus receive-time semantics and loss before reachability.
- [Native producer/report ports](../../../.docs/observability/runtime.md#native-producer-and-report-ports): RuntimeRecorder/RuntimeReporter and concrete inputs/updates, truthful queue publication ordering, independent worker/dispatcher reclamation and exact host publication ownership. ConfigPeerBinding aliases AcceptedPeer; ConfigurationRequest/Response/Error match the configuration owner's released facade.
- [Event contract](../../../.docs/observability/runtime.md#runtime-event-contract): RuntimeEvent/RuntimeCorrelation/RuntimeFact import shared ObservationTime/epoch. Intake constructs the Events-owned ClientReportObservation wrapper retaining submitting AcceptedPeer; it cannot publish caller-selected server/domain facts or generic HostEffectReport. Dataset journal excludes pre-generation startup facts while runtime snapshot remains available.
- [Lifecycles](../../../.docs/observability/runtime.md#state-transitions-and-failure-meaning), [bounds](../../../.docs/observability/runtime.md#bounded-state-maintenance-and-concurrency), [fixtures/handoff](../../../.docs/observability/runtime.md#design-fixtures-and-implementation-handoff): R-01 through R-12 map concrete normal/failure/pressure/reset/restart/configuration cases to OBS-01 through OBS-10 and name downstream native leaves.

Architecture, configuration, events and independent integration received usable releases through their actual Paseo bindings. Direct consumer findings above were resolved in this owned design; no shared/sibling file was edited. Source authority and planned source homes use the architecture release rather than duplicate shared definitions. There is no remaining known cross-owner contract blocker in this Task.

## Evidence, limits and resource release

Completed evidence is read-only native source analysis plus authored typed declarations, transition tables, bounds and design fixtures. No Rust/Node/Cargo build, tests, product/runtime/platform probe, live/default scope operation, package/dependency/artifact change or additional review workflow ran. Independent integration owns the limited document/contract checks and cross-domain design acceptance. Rendering, actual 32/16 saturation isolation, passive filesystem behavior, native endpoint/authentication/cleanup, report capture loss and timing/memory claims still require the named downstream implementation/integration Tasks; this completion does not certify any of them.

The provider-error recovery resumed the existing assignment and draft; it did not repeat implementation or alter authority. Owned writes to runtime.md and this Task are released. No terminals, background jobs, listeners, leases, locks, test directories, native resources or subagents were created or retained. The next owner action is architecture/integration consumption and design Delivery closure; future implementation remains a separate planned Delivery.

