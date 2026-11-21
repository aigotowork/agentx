---
schema: task
status: completed
agent: Observation stream implementation owner
---

# Volatile observation journal and truthful continuation

## Candidate-02 bounded correction

Independent integration returned one Events test-only `field_reassign_with_default` diagnostic at `journal_tests.rs:277`, preserved in `/tmp/agentx-obs-native-8hyqm9g2/observation-clippy-02.log`. This Task is active for a bounded transfer of only `src/observability/journal_tests.rs` and its own status record: initialize the request filter directly in its initial struct literal while preserving the test's later intentional mutation. Candidate-02 and prior evidence remain untouched. No owner formatter/compiler/test/probe execution is authorized; next condition is exact corrected test-file release to integration.

Correction complete: `server_request_filter_is_collection_daemon_qualified` now initializes `ObservationFilter { request: Some(RequestObservationId(1)), ..ObservationFilter::default() }` directly. Its later `request = Some(RequestObservationId(2))` assertion remains unchanged. Only the transferred test file and this original Task changed; production modules, frozen inputs and logs remain untouched. The same 21 cases remain applicable, with no execution by this owner. The bounded correction releases its writes/resources; integration alone formats and validates the changed derivation. No new passing-check claim is made.

## Candidate-01 bounded correction

Independent native integration returned 17 Events diagnostics from strict Clippy on the fixed 445-input candidate `/tmp/agentx-obs-native-8hyqm9g2/observation-candidate-01-source`, with zero drift; exact diagnostics are preserved in `observation-clippy-01-events.json` and complete explanations in `observation-clippy-01.log` in the same evidence root. Its central formatting and DB ownership checks passed; compilation/Clippy failed and no tests/package ran. This Task is active only for the transferred original Events source/test files and this status record. Current repair: explicit native test imports, MSRV 1.89-compatible checked atomics, inline-envelope/dead API rationale, and collapsed nested conditionals. Public payload shapes, serde representation, budgets and behavior remain unchanged. Next condition is corrected source release to the sole integration executor; no owner formatter/compiler/test/probe execution is authorized or claimed.

Correction release: added explicit `messaging::inbox::{BatchId, ClaimPhase, EventId}` imports in `journal_tests.rs`. `journal.rs` funnels both checked atomic counter additions through one private `checked_atomic_add` using the original standard operation and AcqRel/Acquire ordering. Only that helper carries a documented deprecation allowance because newer `try_update` is absent on MSRV 1.89; no custom CAS loop, unchecked arithmetic or API upgrade is introduced. Collapsed the reported publish/suppression/filter nested conditions using MSRV-compatible let chains. Four exact enum declarations in `events.rs` retain the accepted inline public value contract with local, reasoned large-variant lint allowances; boxing would change native construction/matching APIs without a behavior need. Two exact optional/recovery methods retain reasoned dead-code allowances because production composition does not presently call them; their authored behavioral cases remain applicable. No global lint suppression, formatter configuration or external source is changed.

The three originally owned Rust files plus this original Task are the entire correction release. The same 21 cases and `observability::journal::tests` filter remain applicable and unexecuted by Events; no added implementation-mirroring tests or owner checks were run. All frozen candidate/evidence files remain untouched. This bounded repair is complete and releases its writes/resources again; independent integration must format the corrected leaves, preserve the changed derivation, and determine whether strict Clippy and the selected cases now pass. Candidate-01's failure remains real historical evidence, not a claimed pass after this source correction.

## Authority and entry

Read the root `AGENTS.md`, the [observability team entry](../AGENTS.md), [implementation Delivery](../implementation.md), [feature entry](../../../.docs/observability/README.md), [engineering map](../../../.docs/observability/engineering.md) and the specific contracts below. The user has authorized the implementation Delivery. Follow the [current implementation team contract](../implementation-team.md) and [actual assignments](../controller.md#implementation-assignments); this Task becomes active only under its assigned owner. Preserve the completed daemon baseline; execution uses explicitly disposable scopes and does not include installed/live cutover.

## Inputs and result contract

[Event stream design](../../../.docs/observability/events.md), [shared epoch](../../../.docs/observability/contracts.md#shared-references), OBS-03/05/08/09/10 and the runtime/configuration public payload offerings.

Implement a bounded best-effort observation journal and typed Describe/Read contract. Preserve stream nonce in `ObservationStreamId` separately from shared `ObservationEpoch`; preserve fixed through frontier, scanned continuation, exact gaps, era/collector changes and out-of-band collection losses. Durable domain event/receipt authority remains unchanged.

## Write and resource boundary

Own `src/observability/{events.rs,journal.rs}` and named tests. Own event design promotion and native guide sections. Existing `src/events/`, SQLite event storage and business commits stay with their current domains; composition owns any postcommit reference hook.

## Required work and decisions

Implement selected record/byte/age/page/scan budgets and nonblocking producer insertion without turning collection failure into business failure. Import runtime/configuration payload declarations. Use fresh `OutputObservationId` and safe `DeliveryBoundary`/`ConfirmationDisposition` projection; telemetry joins must not reconstruct native output/receipt mutation tokens. Suppress detailed instrumentation of observation infrastructure to prevent recursion. Short passive reads retain no manifest/pin/subscription; timer/backoff belongs to consumers.

## Authored cases and acceptance

Author selective no-match with scanned progress; matching record blocked by response budget; concurrent append above frozen through; retention gap between pages; complete expired window; future/inverted input; same epoch/new stream nonce; restart/reset; contended/oversize/retired drops; same `(stream,sequence)` replay and pressure. Ensure capture drops without assigned sequence remain distinct from a retained-prefix gap.

The [Delivery](../implementation.md) owns formal stage dependencies. Native/domain owners author cases; the assigned independent integration owner executes the necessary checks. Keep source analysis, authored fixtures, executed evidence and installed support distinct. Do not rerun a passed check without a changed relevant input or explicit requirement.

## Handoff and completion

Release native sink/query implementations, payload mapping and authored scenarios to composition/diagnostics/native integration. Supply exact postcommit and validated report insertion instructions. No executable backend acceptance is inferred from in-memory fixture success.

Maintain this original Task's status and source/evidence basis. Replace promoted design declarations with native links, update the owning guide, identify remaining limitations, and release exact writes/processes/endpoints/roots before completion. No installed/live release is implied.


## Implementation activation

The user authorized this implementation batch after accepted design. The actual binding and source/resource scope are recorded in the controller. Initial work: Prepare native payload/sink/read declarations, journal skeleton and cases; consume exact shared/runtime/config refs. Keep safe projections and retained gaps authoritative. Maintain further source releases, real blockers and next action here. Downstream callable dependencies and independent execution remain required for acceptance.

## Working record

2026-11-08: Entered the new implementation assignment; the old design Task stays completed. Structure reuses the accepted design: `events.rs` owns public sink/read/payload declarations and validation/matching; `journal.rs` owns bounded serialized retention, stream-fenced producers, nonblocking synchronization and checked counters; `journal_tests.rs` owns selected meaningful cases. Existing source/DB writes remain composition-owned, including bounded after-commit/report hook insertion. Preparation is active while shared native context is being materialized. Runtime/configuration declarations now exist and the event skeleton imports them directly. No compilers, formatters or tests are run by this owner. Paseo suggestion `UNS-9` tracks this implementation separately from completed design.

Native shared `context.rs` is now consumed directly, including `ObservationClock`, validators and serde. Public event/payload/sink/reader declarations were released early to Runtime/Configuration/composition. The actual journal behavior now includes fixed stream handles with weak owner refs (old handles cannot pin old ring buffers), checked counters, 8,192/16 MiB/15 min prefix retention, capped encoding and bounded Describe/Read with exact frontier/gap/era results. Producers use a try-lock and actual Runtime/report validation, and retained records are immutable encoded bytes. Composition received constructor, publish/unpublish/refresh-sink instructions and exact postcommit boundary; its existing-source hook work remains independent.

Final source-boundary inspection separated poisoned collector failure from retryable contention, retained explicit replacement after sequence/counter exhaustion, and moved production collection/head clock sampling under the collector lock. Runtime received and applied the concrete missing optional-attempt UUID validation at its own helper; Events validates only its owned domain/filter/config reference crossings. The scoped case leaf and guide/design promotion now exist. Owned source/context implementation is complete and released; the next executable condition is composition's coherent release plus independent integration's fixed-candidate execution. There is no remaining owned blocker or passing execution evidence.

## Native callable release

- [events.rs](../../../src/observability/events.rs) owns exact sink/reader/request/result/payload/filter/error/loss declarations, safe domain projections, validation, matching and derived severity. It imports actual shared, Runtime and Configuration public contracts; no duplicate types or generic JSON payload is introduced.
- [journal.rs](../../../src/observability/journal.rs) implements `ObservationJournal`, re-exported by `crate::observability::events`. Its native `new`, `publish_epoch`, `unpublish_epoch`, `replace_stream`, `sink`, `prune`, `close`, `suppress` and `handle` methods are behavior-callable source. `ObservationReader::{describe,read}` supplies short typed reads. There is no successful stub, disk persistence, per-reader registration or pin.
- [journal_tests.rs](../../../src/observability/journal_tests.rs), filter `observability::journal::tests`, contains 21 owner-authored cases: selective no-match/scanned bound, fixed through/replay, byte-blocked match, age/count/byte pressure and exact gaps, completed expired window, invalid/future boundaries, reset/restart/nonce, drops before sequence assignment, weak-handle lifetime, safe output projection, daemon-qualified server request, historic report versus current AcceptedPeer, config source/epoch, overflow/replacement, poison, terminal close and nonrecursive suppression. These cases use pure value fixtures, no filesystem setup, sleeps, services or external scopes. They are authored and unexecuted by this owner.
- [Event model](../../../.docs/observability/events.md) now links native declarations and retains semantic fixtures, receipt/evidence limits and exact retained-window behavior. [Native guide](../../../src/observability/AGENTS.md#events-maintenance) routes maintenance and lifecycle/producer obligations. The completed design Task and design acceptance remain historical.

## Composition mapping and lifecycle handoff

Composition binding `b56613b3-d267-4b49-996b-f80a3fbec937` is the sole existing-source writer. Import through `crate::observability::events`; never import the private journal directly. Construct `ObservationJournal::new(lease_daemon, shared_clock.clone())` while generation is unknown. The constructor validates identity shape and a 4 KiB daemon-ref encoding bound without resolving or creating a scope. Before publication Describe/Read returns `GenerationUnavailable`; Runtime process diagnostics remain separate.

After the actual committed generation is available, construct the shared epoch and call `publish_epoch(epoch)`, then obtain `sink()` and replace Runtime/Configuration producer handles. Equal healthy epoch publication is idempotent; explicit `replace_stream()` rotates nonce/counters even after exhaustion. Lifecycle changes are serialized by composition. Withdraw with `unpublish_epoch()` before reset/generation-mirror changes; withdrawal takes effect even if cleanup returns `Busy`. Publish the new committed epoch and fresh handles only after mirror publication succeeds. Old handles drop into their own retired counters and never redirect old facts into the new stream. Do not relabel a prepared old-epoch config skip or domain projection. Prune on the daemon's bounded control tick at least once per second, including idle periods; `close()` is terminal and precedes lease release. Poison is `Unavailable`, not a retryable `Busy`; replacement cannot repair a poisoned mutex and requires a new collector under the lifecycle owner.

| Existing authoritative boundary | Safe native mapping supplied to `ObservationSink::record` |
| --- | --- |
| Runtime registry transition after its state update | `ObservationPayload::Runtime(event)` with its actual `correlation.epoch`, `EvidenceOrigin::DaemonBoundary` and exact `event.at`. No-generation process-only events stay outside the dataset stream. |
| Restricted Runtime report intake after admission | Intake constructs Events-owned `ClientReportObservation {report, submitted_by: current_authenticated_peer}`. The receiving current epoch is the input fence; source is `ClientReported` at exact `report.ended_at`. Original daemon/epoch/time remains claimed in the report and never becomes current authentication. There is no arbitrary external event publication endpoint. |
| Configuration store callback after its lock is released | Import exact `ConfigObservationEvent`. Snapshot publication uses its snapshot epoch/original `origin`/`captured_at`; skip/retirement uses the emitting boundary epoch/time/origin. No event pins or reconstructs a snapshot. |
| `db/events.rs::after_event_logged` after native autocommit or deferred outer commit | `DomainCommitFact::EventAppended` with captured scope, generation and actual positive local EventId. Map event kind to Message/Status/Life/Other; do not copy its JSON, body or raw instance into a new participant identity. |
| `messaging/service_store.rs::handle`, `Request::Submit` successful acceptance transaction | **Also emit `MessageAccepted`, not only EventAppended.** Capture `context.caller.request_id`, `context.database_generation` and returned `Acceptance.event`/`replayed`; construct `MessageAcceptanceRef` and `DurableEventRef` with the same captured scope/generation. A replay retains the original event/request with `replayed: true`. `Messages::Acceptance` read alone is not a new commit. If this call is nested, defer publication until the actual outer commit. |
| Successful native inbox Prepare/MarkIssuing/RecordPhase transition | `InboxPhaseStored` with the exact returned/validated target, batch and phase. A rejected operation does not emit a stored transition. Never infer phase from timeout, missing telemetry or current process liveness. |
| Native inbox confirmation returning `Confirmation::Advanced` or `AlreadyConfirmed` | `DeliveryConfirmationStored` with exact target/batch, boundary normalized from HookOutputWritten/ToolOutputWritten/ProviderInputRecorded and matching safe disposition. Omit full receipt, exact through frontier and provider turn identity. WrongBatch/StaleFence/error produces no successful confirmation event. |
| Successful unmanaged `ConfirmOutput`/`ConfirmOutputAndStatus` transaction | `OutputConfirmationStored` with exact native participant/through plus fresh `OutputObservationId::new()`. Never copy/hash native token id or export a reverse mapping. Claimed output follows the delivery-confirmation projection above. |

For every DomainCommitted input, capture epoch and actual available `DomainCorrelation` at the producer boundary; request observation is qualified by that epoch's daemon. Actor is an exact domain-resolved ParticipantRef or None. After actual outer commit, construct source `EvidenceStamp {origin: StoredDomainFact, at: shared_clock.now()}`; this is an observation after commit, not an exact SQLite commit timestamp. Drop a failed clock observation without changing native results. A stale captured epoch remains stale even if publication runs after reset.

The existing `db/mod.rs::{with_immediate_transaction,with_write_scope}` and `db/post_commit.rs` own transaction/savepoint checkpoints. Extend their observation preparation with one shared maximum of 64 projections/64 KiB encoded projection bytes per outer transaction across nested savepoints. Rollback discards the corresponding projections and restores preparation accounting; an outer rollback discards all. Flush outside SQLite only after actual outer commit. Known skipped preparation is charged through `note_drop(PreparationBudget, count)` without blocking; `record` counts its own failure and must not be charged twice. No journal lock spans a transaction and no journal failure alters commit/result/receipt authority. Durable commit and telemetry append intentionally remain non-atomic; the independent lost-reply fixture must still corroborate via real `Messages::Acceptance`.

Full page sizing includes the actual ObservationResponse encoding plus 128 bytes reserved for the fixed passive inspection wrapper. Composition must keep that wrapper within the reserve and existing IPC frame bound. Suppression accounting is best effort when the stream/try-lock is available; it is not an exact inspection traffic metric and emits no recursive event or loss.

## Evidence and receiving condition

Events owner performed source inspection, scenario derivation, native implementation and context promotion only. No Cargo/Node/formatter/build/test/package/probe ran here. No existing business/daemon/client source, dependency, installed program, live service or default scope was changed by this owner. The dirty checkout is preserved.

Independent native integration `c85e6101-0cb7-46a8-824a-2433173743a0` receives the authored module filter, source and mapping above; it owns actual checks, real commit/rollback and lost-reply/passive-lane cases, fixed input evidence and cleanup. Composition owns wiring completion. This local implementation handoff does not claim whole-backend, CLI, adapter/Web, installed or platform acceptance. Event collection remains volatile/selective and best effort; reference presence does not imply universal historical receipt readback or external-work completion.

Completion releases writes to the three native Event/journal/case files, the promoted event model, the Events guide section and this original Task. No file locks, persistent subprocesses, terminals, endpoints, services or disposable roots were acquired or remain. Exact source and mapping are sent directly to composition and native integration; controller receives this completed local handoff. Concrete later diagnostics return to this owner through a bounded correction assignment rather than an unclaimed permanent write lease.
