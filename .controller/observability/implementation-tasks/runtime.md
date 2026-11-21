---
schema: task
status: completed
agent: Runtime and client implementation owner
---

# Runtime/client observation registry

## Authority and entry

Read the root `AGENTS.md`, the [observability team entry](../AGENTS.md), [implementation Delivery](../implementation.md), [feature entry](../../../.docs/observability/README.md), [engineering map](../../../.docs/observability/engineering.md) and the specific contracts below. The user has authorized the implementation Delivery. Follow the [current implementation team contract](../implementation-team.md) and [actual assignments](../controller.md#implementation-assignments); this Task becomes active only under its assigned owner. Preserve the completed daemon baseline; execution uses explicitly disposable scopes and does not include installed/live cutover.

## Inputs and result contract

[Runtime design](../../../.docs/observability/runtime.md), [shared references](../../../.docs/observability/contracts.md) and OBS-01/02/06/07/08/09/10 in [scenarios](../../../.docs/observability/scenarios.md#case-derivation-and-coverage).

Implement the bounded runtime registry, originating-attempt evidence contract and snapshot/detail/report offerings. Preserve independent connection/admission/queue/dispatch/service-result/reply facts, explicit correlation, observed peer versus client claims, and generation-unavailable process diagnosis. A short connection count must never masquerade as persistent host liveness.

## Write and resource boundary

Own `src/observability/{runtime.rs,registry.rs}` and their named native test leaves. Own runtime design declaration promotion and corresponding native module-guide sections. Do not edit `daemon`, `service`, `main.rs`, configuration resolver or business domain implementations: send composition exact producer signatures and insertion points.

## Required work and decisions

Consume exact released runtime declarations, limits, errors and publication ordering. Publish read/producer handles that inspection can use independently of Application/SQLite. Keep update cost and memory bounded; report pressure and missing evidence with the selected semantics. Import configuration refs and event sink contracts from their owners, never private stores; coordinate additive declaration releases where necessary.

## Authored cases and acceptance

Author connection without Call, pre-server local failure, Busy before admission, deadline before dispatch, timeout after dispatch, service result with failed reply write, same PID/different or unavailable incarnation, reset/restart, bound exhaustion and release/cleanup cases. Include congestion while an ordinary application call remains held; native registry cases alone do not prove transport lane isolation.

The [Delivery](../implementation.md) owns formal stage dependencies. Native/domain owners author cases; the assigned independent integration owner executes the necessary checks. Keep source analysis, authored fixtures, executed evidence and installed support distinct. Do not rerun a passed check without a changed relevant input or explicit requirement.

## Handoff and completion

Release callable query/producer implementations and authored cases to composition and native integration. Supply exact `server`/`transport`/`client` hooks and lifecycle stop/drop ordering as instructions, not shared-file edits. Report source/runtime evidence separately.

Maintain this original Task's status and source/evidence basis. Replace promoted design declarations with native links, update the owning guide, identify remaining limitations, and release exact writes/processes/endpoints/roots before completion. No installed/live release is implied.


## Implementation activation

The user authorized this implementation batch after accepted design. The actual binding and source/resource scope are recorded in the controller. Initial work: Prepare runtime public declarations, skeleton and meaningful native cases; consume actual shared imports before filling dependent behavior. Publish peer/config/event requirements directly to their owners. Maintain further source releases, real blockers and next action here. Downstream callable dependencies and independent execution remain required for acceptance.

## Current work

Candidate-01 strict Clippy source repair is completed and released under native integration's bounded transfer. All 21 supplied runtime diagnostics have a targeted source correction; this is not a claim that the corrected candidate passes. Exact diagnostics: `/tmp/agentx-obs-native-8hyqm9g2/observation-clippy-01-runtime.json`; frozen 445-file source and full log remain integration-owned and untouched. Next condition: native integration formats corrected runtime.rs/registry.rs and freezes the changed derivation for its necessary checks; no owner-executed formatter/compiler/test/probe.

Owner binding: `{serverId: srv_yDbm9zuqIMiM, agentId: 4e409b53-d9db-4257-930f-8011ab8b539e}`; board mirror UNS-10. Accepted design is the structural model: `runtime.rs` owns public runtime facts/ports and origin evidence helpers; `registry.rs` owns finite retained/active state, transitions, metrics, short reads, report intake and queue/RAII synchronization. No broad redesign is needed.

The initial native result and the subsequent bounded clock-unavailable correction are completed and released. Actual shared context, configuration refs and event sink/payloads are imported. All owned writes/resources are released again. The next condition belongs to composition/native integration: wire the real hooks and optional local-failure time, freeze the coherent graph and execute the authored cases and transport scenarios. No native build/test or actual endpoint support is claimed by this Task.

### Native implementation progress

Actual shared `ObservationClock`/identity/evidence, configuration refs and event sink/payload declarations have now been consumed from their native owners. `runtime.rs` carries the promoted public types, validation/classification helpers, capped encoded-size counting and bounded local report retention; `registry.rs` materializes the Arc/Mutex state, independent execution/wait/write updates, finite active/history/projection/report retention, current-era/lifetime metrics, short try-lock reads, peer-bound report intake, epoch/sink publication and queue/RAII synchronization. `registry_tests.rs` contains meaningful authored native cases; none has run.

Composition receives the real constructor and producer signatures, queue publication gate and connection/worker/dispatcher guards. Runtime declarations are promoted to native source links in [runtime design](../../../.docs/observability/runtime.md#native-implementation-release-and-composition-instructions), and the runtime section of [native module guidance](../../../src/observability/AGENTS.md#runtime-maintenance) explains ongoing ownership and invariants. This is implemented source behavior rather than a transport-isolation or passed-native-check claim. Only independent integration may execute checks.

## Final native release

| Source authority | Concrete result and consumer |
| --- | --- |
| [runtime.rs](../../../src/observability/runtime.rs) | Crate-private runtime IDs, facts, snapshots, queries/results/errors, origin reports, RuntimeRecorder/RuntimeReader/RuntimeReporter, validation/classification helpers and LocalAttemptReports. Re-exports RuntimeRegistry and its guards from the private implementation; all consumers import this leaf. |
| [registry.rs](../../../src/observability/registry.rs) | Behavior-implemented cloneable RuntimeRegistry, finite state/metrics/history/projections/report intake, short try-lock readers, publication ordering and RAII cleanup. Imports actual ObservationClock/shared evidence, configuration refs and public ObservationSink/ClientReportObservation. No Application, SQLite or transport ownership. |
| [registry_tests.rs](../../../src/observability/registry_tests.rs) | Thirteen authored state cases, available to independent integration under `observability::registry::tests`; unexecuted by this owner. |
| [Runtime native handoff](../../../.docs/observability/runtime.md#native-implementation-release-and-composition-instructions) | Exact server/admission/Call/queue/dispatcher/Application/reply/host/passive-lane insertion table and source/evidence limits. |

Construction: `RuntimeRegistry::new(DaemonRef, ObservationClock, Option<Arc<dyn ObservationSink>>) -> Result<RuntimeRegistry, ObservationValueError>`. Composition creates the process clock once and supplies clones. `clock()`/`now()` expose that actual origin. `generation_published` accepts the actual committed ObservationEpoch; `generation_unavailable` preserves process diagnosis without fabricating dataset identity; `set_sink` replaces the journal's fixed-stream handle after rotation. No old request is relabeled into a new generation.

Producer entry points are the native RuntimeRecorder methods: lifecycle/admission_tick/dispatcher_progress/generation_published, connection_opened/peer_observed/connection_changed, call_decoded/request_changed, host_validated and maintain. The passive facade calls RuntimeReader::read or RuntimeReporter::report_client with separately constructed AcceptedPeer. `snapshot` is a typed convenience read. Origin reports remain claims and use the Events-owned wrapper containing the current authenticated submitter.

Ordering and cleanup: `publish_queued(optional_ref, || sender.try_send(job), classify_rejection)` gates only nonblocking queue publication and its Queued/AdmissionRejected fact. `request_changed` uses the same gate before Dequeued/DispatchStarted; release it before Application. Carry the optional request ref independently in worker and Job. `worker_guard` releases only the reply-worker pin; `dispatch_guard` marks unexpected interruption unless `finish()` follows the explicit rejection/result callback. Active refs survive until both owners finish. A missing observation reference never rejects or cancels native work; if later transitions cannot be accounted, registry reads become Unavailable rather than reporting false idle. `connection_guard(Business|Inspection)` releases observation/gauges on Drop; `close_as` records the true terminal reason without owning the stream. Composition closes/joins the real inspection lane before releasing ScopeLease.

## Working method and source basis

Structure-first implementation follows the accepted domain split: public values/ports in runtime.rs, bounded state in registry.rs and owner-local cases in registry_tests.rs. Shared native imports replaced initial missing-declaration conditions; no parallel private copies or dependency features were introduced. Actual [context.rs](../../../src/observability/context.rs) supplies identity/clock/evidence, [configuration.rs](../../../src/observability/configuration.rs) supplies capture/stage/results, and [events.rs](../../../src/observability/events.rs) supplies safe wrapper/sink/disposition. Existing [transport](../../../src/daemon/transport.rs), [server](../../../src/daemon/server.rs), [client](../../../src/service/client.rs) and service ownership remain composition-owned; the insertion table identifies their exact required boundaries instead of editing those files here.

The synchronization decisions preserve separate native execution, reply-wait and reply-write facts, including a late Application result after timeout and failed write. Sixty-four active references cover the selected finite business resources even when a 2 KiB detail is oversized. History is capped at 1,024 rows/2 MiB/5 minutes; peer and host projections each at 256 rows/256 KiB with their selected lifetimes; intake reports at 256 rows/1 MiB/5 minutes with 4 KiB source validation; origin-local reports at 32 rows/128 KiB. Lifetime counters survive reset; only source-era current-window metrics are reset. Checked overflow/poisoning yields unavailable observation, preserving the business result.

Implementation refinements are explicit in the runtime design: bounded encoded-byte counting can occur under the short state lock without allocating an output copy or performing external I/O; complete event/response serialization stays outside. Report duplication uses typed equality instead of serialized copies under the lock. The native mutex's poisoning already makes reads Unavailable; no separate exported fault counter is claimed. Events' concrete final correction is applied: RuntimeEvent::validate checks optional client_attempt UUID-v4/RFC4122 shape even for direct Rust construction, and an existing authored case covers a nil UUID. No shared semantic incompatibility remains known at release.

## Authored evidence and remaining execution

The thirteen cases cover Hello without Call/inspection cleanup, admission Busy and queued deadline NotRun, timeout plus later result plus failed write, incarnation grouping/unknown incarnation, queue publication racing dispatcher, reset/restart fencing and recent/lifetime counters, oversized detail with active gauges, peer-bound report intake/conflicting duplicate rejection, unwind/TTL cleanup, sink loss/read contention/payload validation, finite connection lanes, bounded originating failure reports without server evidence and missing local-failure time with required report clocks. These are authored fixtures, not passing evidence.

This owner executed **no Cargo, Node, formatter, test, build, package or runtime/platform probe**. Source inspection and exact public-contract consumption support this release; compiler/type correctness, insertion coverage and actual behavior remain to be established by native integration on its fixed candidate. In particular, held Application plus 32 business workers/16 jobs with useful independent inspection, passive filesystem behavior, authenticated peer acquisition, real client attempt capture, reset wiring and endpoint/process cleanup require composition/integration evidence. Native state cases alone cannot establish those properties. Installed binaries, PATH, default scopes, live services, remote peers and unexecuted platforms are outside this result.

## Release and next owner

Runtime-owned source, tests, declaration promotion, module-guide section and this original Task are released to the named composition/native integration consumers through their controller bindings; configuration/events consume the exact native public values and validators. Board mirror UNS-10 moves to in_review for the independent execution step. No source insertion file, dependency, old completed Task/Delivery or sibling domain implementation was changed by this owner. No process, listener, endpoint, temporary root, package artifact or other runtime resource was created or retained. There is no current owner-local blocker. Further work is limited to concrete integration diagnostics assigned back to this same native scope.

## Clock-unavailable local failure reconciliation

Composition reported through architecture a concrete incompatibility after the initial release: mandatory LocalFailure.at cannot represent inspect_existing/local_failure when ObservationClock::now returns TimeOverflow. The [bounded architecture decision](native-contracts.md#clock-unavailable-local-failure-reconciliation) selects optional capture time without changing the adapter result shape. This is a correction to the same native Task, not a reopened design Delivery or general review.

Implemented in runtime.rs: LocalFailure.at is Option<ObservationTime>; LocalFailureKind includes ClockUnavailable. An independently known transport error retains kind/stage/os_code even when timestamp capture fails. The diagnostic clock's own failure has no timestamp or OS code. None proves no ordering, elapsed time or age. ClientAttemptReport::validate checks a present failure time against the required report clock/range and rejects any ClockUnavailable timestamp/OS code. Report start/end/span times remain required; no fallback time or new origin admits an incomplete report. Registry state/clock/counter/sink behavior is unchanged.

The existing runtime report fixture uses Some(at). The thirteenth authored case, `observability::registry::tests::missing_local_failure_time_preserves_cause_without_relaxing_report_clocks`, covers missing-time preservation of a connection error and OS code, valid present time, out-of-range/wrong-clock rejection, valid ClockUnavailable, contradictory timestamp/OS-code rejection and continued required report clock consistency. It is unexecuted. Runtime semantic documentation and native module guidance now state the same limits.

Composition retains sole writes to daemon/observation.rs and removes its fabricated maximum-time fallback using this released native shape. Integration receives the exact added case and source impact. No formatter/compiler/test/probe ran, no other domain/shared source was edited and no resource was acquired; this correction's writes are released at handoff. There is no actual unresolved incompatibility known in the selected minimum correction.

## Candidate-01 strict Clippy correction

Independent native integration executed its coherent 445-file candidate with zero drift and returned 21 runtime-owned strict Clippy diagnostics. Authority and evidence are the actual diagnostic list `/tmp/agentx-obs-native-8hyqm9g2/observation-clippy-01-runtime.json` and full log `/tmp/agentx-obs-native-8hyqm9g2/observation-clippy-01.log`; the frozen `/tmp/agentx-obs-native-8hyqm9g2/observation-candidate-01-source` is preserved untouched. Integration reports central formatting and DB ownership passed for candidate-01, while strict Clippy failed; no test/package evidence was supplied. This concrete failure authorized only the original runtime source/test repair, not broad cleanup or a baseline rerun.

Corrected runtime.rs and registry.rs in the current checkout only. Seven unused-import/dead-API diagnostics are addressed by narrow `allow` attributes with reasons on the intentional public guard reexports, clock/snapshot conveniences, RuntimeCapture coverage/skip contract, borrowed-reference helper and local report consumer methods. Native offerings are retained rather than changing signatures to match current incidental use. RuntimeRequest keeps the bounded typed report inline with a justified large_enum_variant allowance; RuntimeResponse's unfulfilled expectation is removed because the actual candidate does not trigger that lint. The remaining twelve collapsible-if diagnostics are corrected with equivalent short-circuit/let-chain guards, including the two diagnostics on one nested queue histogram path. Lock scope, overflow accounting, ownership, native outcomes, wire shape and report/retention semantics are unchanged.

No tests were added for equivalent guard/lint-only changes; all thirteen existing meaningful cases under `observability::registry::tests` remain applicable and unexecuted by this owner. Source inspection checked the supplied locations and corresponding complete log explanations; no Cargo, formatter, compiler, test, package, Node or probe ran. Native integration alone formats and executes the corrected derivation, preserving candidate-01 failure evidence. Exact correction release goes directly to that owner and composition/controller; runtime source/test writes are released again, with no resource retained and no current owner-local blocker.
