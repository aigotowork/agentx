---
schema: document
status: active
owner: observability-integration
last_updated: 2026-11-07
description: Independent design-contract acceptance, resolved findings, exact input basis and evidence limitations.
---

# Observation design acceptance

This ledger is limited to the [observability design Delivery](../../.controller/observability/delivery.md). Its original [integration Task](../../.controller/observability/tasks/integration.md) owns execution state. It does not claim a working observation executable, installed service, browser API or rendered management page. Existing daemon implementation/evidence remains with the [completed Delivery](../../.controller/domain-daemon/delivery.md#accepted-local-outcome).

Current executable support is recorded separately by [native evidence](../../.controller/observability/implementation-evidence/native.md#final-independent-backend-handoff) and [consumer evidence](../../.controller/observability/implementation-evidence/consumer.md), under the authorized [implementation Delivery](../../.controller/observability/implementation.md). The historical design receipt and input identities below remain unchanged; actual HTTP/browser interaction does not turn this static receipt into executable evidence.

## Selected input basis

| Input and authority | Consumed meaning |
| --- | --- |
| [Selected Proposal](../proposals/agentx-observability.md), [design Delivery](../../.controller/observability/delivery.md) and [team contract](../../.controller/observability/AGENTS.md) | Complete model/contracts/state/governance before Web; design-only authority and original specialist ownership |
| [Shared contracts](contracts.md), [model](model.md) and [engineering map](engineering.md) | Scope/daemon/dataset/clock/evidence construction, separate stream identity, same-lease passive lane, native home and shared writer ownership |
| [Runtime/client offering](runtime.md) | Final released [identity](runtime.md#runtime-identity-contract), [passive adapter](runtime.md#passive-inspection-and-structured-local-failures), [facts](runtime.md#request-connection-and-host-facts), [read/metrics](runtime.md#snapshot-metrics-and-read-contracts), [origin reports](runtime.md#originating-attempts-and-report-admission), [producer ports](runtime.md#native-producer-and-report-ports), [events](runtime.md#runtime-event-contract), [transitions](runtime.md#state-transitions-and-failure-meaning), [bounds](runtime.md#bounded-state-maintenance-and-concurrency) and [R-01..12](runtime.md#design-fixtures-and-implementation-handoff) |
| [Configuration offering](configuration.md) | Final released [source rules](configuration.md#resolution-rules-to-preserve), [capture refs](configuration.md#capture-and-snapshot-references), [values/source receipts](configuration.md#safe-values-source-reads-and-resolution), [subjects/resolution scope](configuration.md#snapshot-subjects-and-provenance), [disclosure](configuration.md#disclosure-policy), [producer/capture handle](configuration.md#capture-and-publication-operations), [finite facade/reader/compare](configuration.md#retained-lookup-and-comparison), [bounds](configuration.md#state-maintenance-and-boundedness) and [scenarios](configuration.md#focused-scenarios-and-implementation-obligations) |
| [Events offering](events.md) | Corrected shared epoch/native home, finite Describe/Read, typed envelope/errors/losses, safe domain projections, retention transitions and typed read traces |
| [Current source](scenarios.md#investigation-model) and [baseline-drift disposition](../../.controller/observability/feedback.md#baseline-documentation-drift) | Source-owned caller/daemon/event/config/domain facts; controller corrected stale model/roadmap prose without changing native behavior |

These are the final actual owner releases consumed on 2026-11-07. Each original foundation Task records completion and released writes/resources; no known cross-contract blocker remains. The exact sections above are current authorities rather than source copies. No runtime pass is inferred from a link or digest.

## Findings and dispositions

| ID | Trigger and consequence | Owner and concrete disposition |
| --- | --- | --- |
| INT-01 | Existing `connect_existing` uses scope preparation that creates directories; ordinary Session may resolve/bind identity and normal command feedback changes product state | Runtime and architecture received concrete source references. Selected design uses a passive scope/inspection lane independent of ordinary acquire/Session/Application/SQLite; runtime exact release remains an acceptance input. |
| INT-02 | Early Events draft defined nonce-bearing `ObservationEpoch` and `src/observation/events/service.rs`, incompatible with shared types/module map | Events owner corrected to imported shared `ObservationEpoch`, own `ObservationStreamId { epoch, nonce }`, native `src/observability/events.rs`, and distinct `StreamChanged`/`EpochChanged`. Architecture confirmed sole shared authority. |
| INT-03 | Early event confirmation projection could reconstruct a native confirmation command through full receipt or output-token identity | Architecture and Events resolved at owner boundaries: observation-only output identity and safe `DeliveryBoundary`/`ConfirmationDisposition` projection, without complete receipt/frontier/turn/confirmation token. OBS-03 consumes the corrected declaration. |
| INT-04 | Historic model/roadmap DB-open/daemon-target prose contradicted accepted current baseline | Architecture reported it; controller made a bounded source-linked correction and owns [disposition](../../.controller/observability/feedback.md#baseline-documentation-drift). Include those controller files in the same final document check. |
| INT-05 | Runtime declared a mandatory `ObservationTime` deadline while its expired decode branch requires unavailable evidence | Runtime corrected the field to `EvidenceValue<ObservationTime>` and defined expired-guard `Unknown(NotCaptured)`. Independent wait/write dimensions now also preserve timeout followed by reply-write failure. |
| INT-06 | Runtime inspection imported missing configuration facade types; config peer binding/capture allocator and startup stage were not yet concrete | Owners released matching `ConfigurationRequest/Response/Error`, exact `AcceptedPeer` alias, `StartupInputs` and concrete clone-safe `InvocationCaptureHandle`. Native recorder/reporter ports and same-lock expected-epoch admission are explicit. |
| INT-07 | Literal Unix `observe.sock` sibling would collide across scopes in the actual `/tmp/agentx-{uid}/{digest}.sock` layout | Runtime now selects `/tmp/agentx-{uid}/{scope_digest}.observe.sock` and scope-keyed Windows suffix. Passive lookup does not create/repair the base directory. |
| INT-08 | Stale expected daemon cannot be represented by Events Describe or Config Head(None) leaf epoch errors; shared catalog could drift from the reader's peer argument | Runtime released route-neutral `InspectionResponse::DaemonChanged { expected, actual }` before any leaf, explicit passive entry functions, and `RuntimeReader` with transport-supplied AcceptedPeer. Architecture links the exact native declarations; scenarios handle the neutral branch first. |
| INT-09 | Same-epoch configuration collector replacement could reuse sequence refs; retaining reported snapshots in one epoch did not establish their evaluated root | Configuration preserves allocator/frontier within an epoch, blocks overflow until a new epoch, adds `resolution_scope` and `ScopeUnestablished`, and returns stage/purposes in ComparisonBasis. Events qualifies local request sequences with stream daemon. |

All listed incompatibilities have concrete owner corrections. None was resolved by inventing consumer-only shared types or weakening unknown outcomes. Architecture independently found several of the same boundaries; all corrections belong to this same design-consumption closure. Provider high-demand interruptions are recorded by the controller and original owners; resumed actual releases supply acceptance, not the interrupted drafts.

## Professional scenario consumption

The [canonical scenario source](scenarios.md) owns actors, steps, structured fixtures and consequences. The initial OBS-01..10 derivation preceded final specialist solutions. Final exact runtime/configuration/events and existing native domain branches have now been consumed with all owner corrections above. This is professional static reasoning, not executed product, compile or finite-model evidence.

| Scenario | Deciding consumed boundary | Design result and future enforcement |
| --- | --- | --- |
| OBS-01 unreachable daemon | Passive ExistingScopeInput/ScopeLookup, LocalFailure on ClientStage, originating report knowledge | Consumable without server evidence; native composition/CLI must prove no-start/no-mkdir and preserved failure stage. |
| OBS-02 congestion/slow requests | Separate <=2-worker inspection, runtime activity/gauges, queued/dequeued/NotRun edges and independent reply wait/write | Queue, execution and reply meanings compose; actual native lane isolation/producer ordering remain implementation tests. |
| OBS-03 uncertain mutation/delivery | Client MayHaveBeenWritten, native acceptance/outstanding reads, postcommit safe evidence refs and boundary projection | No false no-effect/replay inference or mutation-token reconstruction. Missing historical receipt readback stays explicit. |
| OBS-04 configuration difference | Immutable actual-use refs/resolution scope, capture handle/facade, safe source trace and typed compare | Historical versus current reevaluation, hidden equality, invalid fallback and unavailable operands are consumable. Actual same-pass loader capture remains implementation work. |
| OBS-05 backlog/gap | Frozen through, scanned next, exact Gap, no-match and separate sampled pre-sequence losses | A complete truthful continuation exists without pins/subscriptions; executable byte/scan/interleaving cases remain required. |
| OBS-06 aliases/multiple scopes | Passive canonical scope and scope-keyed endpoint; incompatible native refs/selection errors | Aliases join only after real canonicalization; independent scopes have no accidental endpoint/ref collision. |
| OBS-07 PID reuse/host relations | AcceptedPeer/PeerObservationRef, unknown-incarnation isolation and last-validated host projection | Numeric PID/closed sockets cannot establish host liveness or identity. Actual OS/platform evidence remains unexecuted. |
| OBS-08 restart | Neutral DaemonChanged before leaf, then EpochChanged after explicit rebind; independent clocks and durable generation | Historical evidence remains labeled; no old observation continuity or new process identity inferred. |
| OBS-09 reset/stale generation | Existing committed generation authority, owner linearization, old-handle rejection and G1/G2 request chronology | Same-daemon process facts can span reset while dataset joins remain fenced; no mixed current batch is promised. |
| OBS-10 pressure/cleanup | Independent registry/config/journal bounds, active pins, finite page/worker admission and best-effort capture | Lost/evicted evidence stays explicit without changing business outcomes. Native memory/timing/cleanup behavior remains to be demonstrated. |

The shared/native module homes, actual producer ports, one composition writer and original implementation Tasks supply an enforceable downstream owner for every selected criterion. The design supports complete investigations whose valid result may be an explicit remaining unknown; it does not promise complete universal observation.

## Machine checks

One bounded document check passed with zero errors after all four final owner releases. The input was 38 actual Markdown/YAML files under `.docs/observability/` and `.controller/observability/`, plus root `AGENTS.md`, `.docs/model/README.md`, `.docs/roadmap.md` and `.docs/proposals/agentx-observability.md`. This includes the controller's baseline corrections, handoff linkage and interruption/disclosure feedback. The final acceptance receipt and integration completion bookkeeping were appended afterward without changing a domain contract or dependency.

| Executed check | Result and interpretation |
| --- | --- |
| YAML parsing and subject/status validation | 37 metadata documents parsed; foundation Tasks completed, ten original implementation Tasks and implementation Delivery still planned |
| Local Markdown links and heading fragments | 529 local links and 152 heading links resolve in the actual workspace; 20 catalog entries exist |
| Delivery decomposition | 10 stages and 15 Task memberships across the design/implementation Deliveries; stage references/conditions valid, no dependency cycles or multiple-stage Task ownership |
| Direct source whitespace and prose checks | All selected files end in newline, have no trailing whitespace; new design/controller prose has no manual paragraph wrapping outside structural/code blocks |
| Tracked diff whitespace | Scoped `git diff --check` passed; direct file checks above also covered untracked new documents |

The one inline Python check used existing YAML parsing and filesystem reads, checked the selected graph/links, and invoked only scoped `git diff --check`; it created no source/test skeleton or helper framework. It is not a compiler, native protocol or runtime guarantee. Markdown heading existence does not establish rendered Mermaid/UI behavior. No Cargo/Node/package/provider/live-service/platform job or previous daemon test was run.

The six final domain-document inputs read by the check have these SHA-256 identities. They identify the consumed bytes, not semantic correctness or executable support; maintained files and canonical sections above remain the recoverable source.

| Input | SHA-256 |
| --- | --- |
| `model.md` | `a4cc5b0d196641934bc7c20ae2dbf8afd10c4019e02654d16ce0fa2e2c60c31f` |
| `contracts.md` | `8c8eba0266b8722b16a1744778ba3efe00ef0af0f72746efc1bd806f96afaac1` |
| `engineering.md` | `8e1e0a40a7e675748773f7f1c06b85f4caa13397500b456e8484ce01ca9feec3` |
| `runtime.md` | `84c4708ccb10b8581417c8a46eb6af636dcd7deca3ea5b8efd1b39f313bfb0a0` |
| `configuration.md` | `b9f2573589531514d6e170c91f271aaa8f94c5478b33dc1e924708a6ec45b0ae` |
| `events.md` | `7d582772285da47075e6c8f70d23dac1d05d31f5afb6a002990de71dab4aca2e` |

## Implementation handoff and limits

The [implementation Delivery](../../.controller/observability/implementation.md) contains ten original Tasks with concrete write boundaries, result contracts, source promotion/context obligations, authored cases and independent execution owners. After this design receipt, the user separately authorized execution and the controller activated the Delivery. The [implementation team](../../.controller/observability/implementation-team.md), [current assignments](../../.controller/observability/controller.md#implementation-assignments), original Task states and [native evidence](../../.controller/observability/implementation-evidence/native.md) are the current implementation authority. [Implementation plan](implementation-plan.md) orders native foundations/composition, passive CLI and independent backend acceptance before local adapter/Web and independent consumer acceptance. The historical machine-check counts, metadata states and hashed inputs above retain their original design-only meaning.

The [consumer boundary](consumer-boundary.md) selects read-only local investigation, one explicit scope, safe disclosure and a later authenticated same-origin local adapter. Global fleet discovery, durable telemetry storage, arbitrary environment/process inspection, new mutation controls, remote browser hosting, vendor/live scopes, installed cutover and unexecuted platforms remain outside this design. Native historical receipt readback is limited to actual existing operations; no universal recovery/history API is promised.

## Independent handoff disposition

The selected model/contracts are consumable for all ten canonical investigations at design scope. All known substantive compatibility findings have owner dispositions; no remaining required design dependency is hidden as future Web work. The implementation Delivery/Tasks are dependency-correct and provide actual native/backend/consumer ownership, meaningful authored cases, independent execution responsibility and context promotion obligations. The controller accepted the design Delivery; its separately authorized implementation now has its own execution and acceptance records.

At design closure, integration released its four design artifacts, planned implementation Delivery/ten Tasks and original integration Task. That design batch acquired no production/dependency/artifact changes, processes, endpoints, local services, test roots, locks or background work. Support at that point remained the completed daemon baseline. Subsequent authorized implementation and executable results belong to the original [native integration Task](../../.controller/observability/implementation-tasks/native-integration.md) and [native evidence](../../.controller/observability/implementation-evidence/native.md); the historical design receipt above retains its original scope.
