---
schema: task
status: completed
agent: Daemon/client composition implementation owner
---

# Daemon and originating-client observation composition

## Candidate-03 byte-pressure fixture correction

Independent integration reports candidate-03 strict Clippy passed and its exact 117 native cases produced 110 passes and seven failures. The one composition-owned failure is `db::post_commit::tests::encoded_byte_budget_can_stop_preparation_before_the_record_count_limit`; its long nested database path causes bundled SQLite error 14 before the scenario, retained in `/tmp/agentx-obs-native-8hyqm9g2/observation-native-tests-03.log`. This bounded correction changes only the cfg(test) fixture/case in `src/db/post_commit.rs` plus this Task. Keep SQLite on the ordinary short temporary path and use a separate long safe native projected scope identity to pressure encoded bytes. Preserve all byte/count assertions and verify admitted facts reach the journal; no production policy or frozen candidate changes, no owner execution/formatting. Integration alone reruns the failed relevant case; 110 successful cases remain preserved.

The fixture correction is source-ready and released. Added cfg(test)-only `fixture_with_scope(root, projected_scope)`; the original fixture delegates with its unchanged actual canonical root. Only the byte-pressure case supplies a long absolute native identity value while SQLite opens the normal short `root/observations.db`; it creates no long filesystem tree. Existing nonzero-count, below-count-limit and <=64KiB assertions remain. Added substantial byte use (>half the byte budget) and exact `journal.newest == prepared_count` assertions, ensuring the input pressures encoded projection bytes and successfully admitted facts publish rather than disappearing through collector rejection. No production declaration/behavior or other owner's source changed. The applicable filter remains exactly `db::post_commit::tests::encoded_byte_budget_can_stop_preparation_before_the_record_count_limit`. Owner has not executed/formatted the correction; integration alone derives the corrected candidate and reruns this failed case. Frozen candidate-03/log and its 110 successes are untouched. All test/Task writes and resources are released again; no pending process/endpoint/fixture root is retained.

## Candidate-01 bounded correction

Independent integration returned ten located composition diagnostics from `/tmp/agentx-obs-native-8hyqm9g2/observation-clippy-01-composition.json`, with full log `observation-clippy-01.log` and immutable candidate at `observation-candidate-01-source` under that same root. The original Task is active only for those existing owned source/test regions. Preserve native consumer/error/wire contracts, correct the nonexistent test method, remove or constrain unconsumed helpers, apply narrow justified finite-envelope lint handling and collapse the inbox conditional. No formatter/compiler/test/probe execution by composition. Integration owns corrected formatting, candidate derivation and execution; prior passing checks are not rerun by this owner.

Correction is complete and source writes are released again. Exactly six current-checkout source files changed: `src/config.rs`, `src/service/observation.rs`, `src/daemon/observation.rs`, `src/daemon/server.rs`, `src/db/post_commit.rs`, `src/messaging/inbox_store.rs`. The nonexistent reset-test call now uses the actual `AgentxDb::get_last_event_id` declaration in `db/events.rs`. Removed the unconsumed `load_env_extras` forwarding wrapper; actual launcher intake still uses the unchanged `load_env_extras_observed`. Retained explicit `drain_local_reports` with a precise dead-code expectation: current passive CLI returns its own attempt without draining unrelated completed origins, and no automatic publication is added. Four finite inspection enums and the shared bounded postcommit enum retain inline native types with narrowly justified `large_enum_variant` expectations. Queue insertion now classifies and drops an unusable rejected Job into the existing Copy RuntimeFailureCode inside the same nonblocking publication closure, eliminating the large returned TrySendError without a new error type, allocation, retry or public contract change. Inbox successful-confirmation observation uses the equivalent short-circuit let-chain.

All ten located diagnostics have a corresponding source correction; no Clippy pass is claimed. No new tests were needed for this mechanical/representation correction. Existing 19 authored composition cases and integration's real saturation/reset/lost-reply cases remain applicable; especially `db::post_commit::tests::reset_created_rows_are_not_published_under_the_admitted_old_epoch`, `daemon::server::tests::queued_request_cannot_cross_reset_or_deadline` and `daemon::observation_acceptance_tests` cover the touched result paths. They remain unexecuted by this owner. Integration alone formats the six corrected files, freezes the changed derivation and executes the necessary checks. Candidate-01 source/diagnostic/log files are untouched; no command/process/endpoint/resource is retained. Later checks and other owners' repairs remain independent of this bounded release.

## Authority and entry

Read the root `AGENTS.md`, the [observability team entry](../AGENTS.md), [implementation Delivery](../implementation.md), [feature entry](../../../.docs/observability/README.md), [engineering map](../../../.docs/observability/engineering.md) and the specific contracts below. The user has authorized the implementation Delivery. Follow the [current implementation team contract](../implementation-team.md) and [actual assignments](../controller.md#implementation-assignments); this Task becomes active only under its assigned owner. Preserve the completed daemon baseline; execution uses explicitly disposable scopes and does not include installed/live cutover.

## Inputs and result contract

Released native runtime/configuration/events implementations; [engineering composition](../../../.docs/observability/engineering.md#dependency-and-composition), [runtime](../../../.docs/observability/runtime.md), [configuration](../../../.docs/observability/configuration.md), [events](../../../.docs/observability/events.md) and [minimum consumer](../../../.docs/observability/consumer-boundary.md#minimum-complete-consumer).

Wire actual originating attempts, daemon/OS stages, effective resolution points and postcommit references into the native stores, and serve passive authenticated inspection under the same daemon lease. Diagnostic admission and reads remain bounded and useful when normal workers/queue/Application are exhausted. Inspection never constructs or accesses managed storage.

## Write and resource boundary

Sole writer of `src/daemon/observation.rs` and additive observation changes in `daemon/{mod,scope,wire,server,transport,platform,windows}.rs`, `service/{context,client,application}.rs`, `main.rs`, actual `config`/launch/host/postcommit producer sites and affected source guides. Existing business owners retain rules; obtain a bounded region transfer for any unrelated concurrent work. Do not change Provider protocols, receipt semantics, settings precedence or reset policy to simplify telemetry.

## Required work and decisions

Use existing native framing/peer authority and exact runtime lane limits. Passive scope resolution must neither mkdir/start/migrate nor fall back to SQLite. Construct observation lifetime before advertising availability and stop/join it before lease release. Supply one daemon clock origin and ClockId to daemon-side producers; clients retain their own clock domains. Apply [epoch publication/concurrent reads](../../../.docs/observability/engineering.md#epoch-publication-and-concurrent-reads): the existing application remains durable-generation authority, and owner reads must yield consistent stamped data or explicit changed/unavailable during reset. Publish unknown generation as process-only evidence. Bind actual resolution facts rather than reconstructing them from current files; publish domain references after actual outer commit. A dropped capture cannot alter product results.

## Authored cases and acceptance

Author actual lane/auth/protocol, unsupported transport, alias/unresolvable scope, initialization/failure, queue expiry, saturated normal admission, held dispatcher, reply loss, postcommit rollback exclusion, reset/restart and cleanup cases. Capture local errors before `ServiceError` flattening. Exact output flush/host evidence remains with the existing domain.

The [Delivery](../implementation.md) owns formal stage dependencies. Native/domain owners author cases; the assigned independent integration owner executes the necessary checks. Keep source analysis, authored fixtures, executed evidence and installed support distinct. Do not rerun a passed check without a changed relevant input or explicit requirement.

## Handoff and completion

Release actual callable native/inspection wire path, insertion source map and authored cases to diagnostics/native integration. Release CLI/main routing to diagnostics after composition is complete. Platform source coverage and actual native execution must remain separate.

Maintain this original Task's status and source/evidence basis. Replace promoted design declarations with native links, update the owning guide, identify remaining limitations, and release exact writes/processes/endpoints/roots before completion. No installed/live release is implied.


## Implementation activation

The user authorized this implementation batch after accepted design. The actual binding and source/resource scope are recorded in the controller. Initial work: Prepare source insertion map and passive adapter skeleton under sole shared-source ownership; use actual released producer contracts and main transfer before dependent edits. Maintain further source releases, real blockers and next action here. Downstream callable dependencies and independent execution remain required for acceptance.

## Working model and source insertion map

Composition preserves the existing singleton/Application authority. One lease-scoped observation aggregate owns the three bounded owners and a separate passive listener; inspection workers receive readers/report intake only. Dataset publication uses one nonblocking reader gate across owner rotation after the durable generation mirror changes. No observation lock spans Application, SQLite, transport or serialization. Initialization/invalid generation remains process-only evidence.

| Source boundary | Additive responsibility | Dependency |
| --- | --- | --- |
| `daemon/scope.rs`, `platform.rs`, `windows.rs` | Read-only existing scope validation and scope-keyed observe endpoint, sharing authenticated peers and inode/DACL ownership | Existing native platform contracts; no foundation dependency |
| `daemon/frame.rs`, `wire.rs`, `observation.rs` | Capped native leaf envelopes, one-second/two-worker passive lane and compatibility/expected-daemon validation | Released native facade and three public leaf offerings |
| `daemon/mod.rs` | Construct before Application initialization; stop/join after Application drop and before lease release, including errors/unwind | Actual owner constructors/clock APIs |
| `daemon/server.rs` | Truthful accept/decode/queue/dispatch/reply stages; separate worker/job lifetime; publish queue before dequeue under short transition gate | Runtime recorder release |
| `service/context.rs`, `client.rs`, `daemon/transport.rs` | Shared invocation revisions and per-attempt local evidence before failure flattening; native authority remains unchanged | Configuration capture and runtime attempt releases |
| `service/application.rs`, config/load/launch/host sites | Capture actual consumed inputs and successful safe host projections at owned resolution/commit boundary | Exact owner insertion instructions |
| `db/post_commit.rs`, `db/events.rs` | Publish safe domain event references only after outer commit; exclude rollback | Events producer release |

Current source basis: `DaemonScope::prepare` and Unix `socket_path` both mutate directories, so neither can back passive inspection. Existing framing allocates a complete 8 MiB frame; the passive lane needs limits before allocation and capped serialization. Existing `serve_with_stop` initializes before listener startup; observation lifetime must surround that initialization without making the ordinary listener falsely Ready. These are bounded composition changes, not new business policy.

Initial preparation established read-only scope validation, scope-discriminated Unix/Windows observe endpoints and capped framing. The scope path does not mkdir/chmod/repair DACLs; Windows receives a read-only protected-user DACL check. Framing enforces the caller's maximum before body allocation and while serializing the complete envelope. The final source release below supersedes this preparation history; no executable evidence is implied.

The three owners have released actual constructors/readers/recorders; shared clock/refs and `main.rs` have been explicitly transferred. The native architecture owner accepted structured transport I/O codes, an adapter publication gate and instrumentation-only scoped per-dispatch context with RAII restore. These are now consumed in source. `daemon/observation.rs` contains the finite native aggregate/passive client, actual store composition, expected-daemon rejection, generation rechecks and separate two-worker lane. `serve_with_stop` retains its public signature and surrounds initialization/Application destruction with observation startup/stop/join under the lease.

Actual daemon queue/dispatch/reply hooks consume RuntimeRegistry's released gate and independent worker/dispatcher guards. `config/observation.rs` supplies same-pass source/field receipts from the existing resolver; it never changes candidate selection or re-resolves values. Original context carries an optional correlation claim only. Per-dispatch configuration buffers until the handler leaves SQLite; domain preparation uses the existing outer-commit/savepoint list. The independent integration leaf remains bound to unchanged lifecycle/passive APIs. All authored cases remain unexecuted; native integration is the sole execution owner.

## Early passive API and CLI region release

The callable source contract in [`daemon/observation.rs`](../../../src/daemon/observation.rs) is stable for diagnostics: `resolve_existing_scope(ExistingScopeInput) -> ScopeLookup`, `inspect_existing(&ScopeRef, Option<DaemonRef>, InspectionRequest) -> Result<InspectionResult, LocalFailure>`, and `originating_clock()`. `InspectionRequest::{Runtime,Configuration,Events}` embeds the original leaf requests. `InspectionResponse::{DaemonChanged,Runtime,Configuration,Events}` keeps the neutral incarnation rejection ahead of every leaf; `InspectionResult` retains `ready`, native response and originating `received_at`. Native `LocalFailure.at` is now optional per the architecture/runtime correction, with explicit `ClockUnavailable` and no synthetic time. An inspection response does not promise atomicity across owners or business readiness.

Composition now releases all additive CLI routing rights in `main.rs`, `cli/{router,commands/mod,commands/help}.rs` and the new observe command to the controller's diagnostics assignment. Composition will not write those regions again without a bounded transfer. Native's `mod observability;` remains present. Diagnostics can consume `service::observation::{AttemptGuard,stage,drain_local_reports}` and `config::observation::NativeConfigResolutionSource` for explicit originating evidence and current reevaluation; neither automatically publishes/retries/opens storage. Command dispatch must precede the existing normal acquisition/dev-root/identity setup according to its owned contract.

Actual origin attempts, immutable invocation revisions, same-pass resolution, postcommit domain references, host authority callbacks and the two-worker passive lane are materialized. A separate EOF result now distinguishes Ready-only probes from truncated frames; the runtime worker pin covers the final write attempt. The existing transaction effect list owns rollback and constant-time 64/64KiB observation budget accounting. Domain projections freeze their epoch/sink and flush only after the real outer commit. Scope/frame and three postcommit cases are authored. Remaining composition work is targeted origin/lifecycle coverage, the remaining actual configuration projection sites and guide/source handoff; this is an early consumable API release, not complete composition, compilation or backend acceptance.

## Composition case model

| Case family and starting state | Actual boundary/input | Expected invariant and owner |
| --- | --- | --- |
| Origin has not written a Call; IO fails | One staged closure returns structured IO before service flattening | Exact stage/kind/raw OS code retained; knowledge remains NotWritten; no raw error text in report. Composition authors `service::observation::tests`. |
| Origin may have written; nested probe/unwind overlaps | Separate RAII attempt and leaf stages | Call uncertainty persists, nested scope restores prior correlation, reports keep one clock and <=12 ordered stages. Same filter. |
| Lease is held; initializer is blocked then fails | Actual `serve_with_stop`, real passive connect | Runtime Initializing with unknown generation, config/events generation unavailable; failure joins listener/workers and releases lease without constructing storage. Composition authors `daemon::observation::tests`. |
| G1 published; read gate overlaps committed G2 mirror | Existing generation publication plus old fixed-stream sink | No leaf returns old current dataset; G1 sink retires; new reads show G2 after the gate releases. Same filter. |
| Authenticated local connection has wrong protocol/scope or no Hello | Real socket peer and production inspection connection parser | Reject before any leaf; inspection creates no business rows/events. Same filter. |
| Actual nested transaction rolls back/commits | Real SQLite/savepoints and original postcommit list | Only surviving references publish, outer rollback publishes none, one shared 64/64KiB bound; rollback restores budget. `db::post_commit::tests`. |

Independent integration's seven native cases already own full 32-worker saturation, held Application, neutral daemon fencing across all leaves, alias/distinct scopes, reset with queued G1 and lost mutation reply reconciled by actual Messages::Acceptance. Reuse that authored coverage; do not duplicate or claim it executed. Scope/frame cases additionally protect passive no-effects, complete-envelope size and clean EOF versus truncated frames. Native Windows peer/DACL/pipe source is separate from runnable Unix evidence.

## Final composition source release

All owned insertion behavior and context handoff are now materialized and source-ready. This Task completes its authored composition offering; independent compilation/execution and complete CLI/backend acceptance remain downstream. The latest source is **uncompiled, unformatted and unexecuted by this owner**. No Cargo, Node, formatter, build, test, package or product probe ran. This is not a pass receipt.

| Actual producer or lifecycle home | Final native consumption and evidence boundary |
| --- | --- |
| [`daemon/mod.rs`](../../../src/daemon/mod.rs), [`observation.rs`](../../../src/daemon/observation.rs), [`scope.rs`](../../../src/daemon/scope.rs) | Actual lease, one shared clock, original safe startup capture, store construction before initialization, separate bounded passive lane, publication/read fences, periodic Runtime/Configuration/Events retention and close/join before lease release. `serve_with_stop` signature unchanged; original integration test module bound. |
| [`daemon/server.rs`](../../../src/daemon/server.rs) | Authenticated business peer; actual decoded call; short queue publication gate; dequeue/validation/dispatch/return; independent reply wait/write and worker/dispatcher guard lifetime; Ready-only EOF classification. No business cancellation, telemetry-driven admission or successful commit inference. |
| [`service/observation.rs`](../../../src/service/observation.rs), [`client.rs`](../../../src/service/client.rs), [`daemon/transport.rs`](../../../src/daemon/transport.rs) | Origin-local bounded attempts before fallible work; clone-safe immutable capture revisions and original RequestId/OriginRef; structured failure before service flattening; no automatic report, spool, retry or raw context retention. |
| [`service/context.rs`](../../../src/service/context.rs) | Instrumentation-only scoped ports with exact frozen refs and RAII restoration; <=8 safe body buffer after handler/SQLite scope; `InvocationContext::capture_observed -> Result<Self,LocalFailure>` shares the ordinary input assembly while preserving original known IO cause with optional time. |
| [`config.rs`](../../../src/config.rs), [`config/observation.rs`](../../../src/config/observation.rs) | Same actual parse/read/select/coerce/normalize/validate trace; same-read InputOnly declarations; native fixed disclosure for effective result. Source receipts cap at native MAX_SOURCE_READS plus one explicit oversize trigger. Explicit current reevaluation uses NativeConfigResolutionSource once. |
| [`runtime/launcher.rs`](../../../src/runtime/launcher.rs), [`service/client.rs`](../../../src/service/client.rs) | Actual launch resolution capture, safe tool/backend/regime/cwd/provider-home plan and original launch-env file read receipt. No env keys/values, arbitrary args, launch JSON or complete consumed-field certification. |
| [`agents/service_store.rs`](../../../src/agents/service_store.rs), [`service/maintenance_store.rs`](../../../src/service/maintenance_store.rs) | Successful explicit Resolve/Record reuses its exact participant row; persistent DevRoot reuses its exact KV result/receipt without extra SQL. Hints fixed withheld. New-dataset reset writes temporarily suppress the old epoch's instrumentation context; outer request chronology and native reset behavior remain unchanged. |
| [`runtime/hosts_store.rs`](../../../src/runtime/hosts_store.rs) | Positive process/native registration and native renew/release follow actual authority/fences and transaction success; absent idempotent release creates no fake positive host. Safe HostReported inputs use actual AcceptedPeer plus validated participant/execution; no registration token enters observation. |
| [`db/post_commit.rs`](../../../src/db/post_commit.rs), [`db/events.rs`](../../../src/db/events.rs), [`messaging/service_store.rs`](../../../src/messaging/service_store.rs), [`messaging/inbox_store.rs`](../../../src/messaging/inbox_store.rs) | Frozen safe EventAppended/MessageAccepted/inbox phase/delivery confirmation/output confirmation facts. Shared 64/64KiB nested budget with constant-time admission and rollback restoration; outer commit only publication; no body/native token/full receipt/provider turn. |

The API/CLI transfer above remains effective; diagnostics `209a7d4f-b655-43d4-9adb-dd858ae8f150` owns main/CLI routing and has consumed `capture_observed`. Its provider-error recovery is controller-owned; composition does not reclaim or race those files. Architecture's optional failure-time decision is consumed without a sentinel. The only later architecture note is the concrete reset observation attribution correction: selective suppression prevents new-dataset rows being falsely labeled with admitted G1 before committed G2 publication.

## Authored filters and evidence limits

Nineteen new composition cases are authored and unexecuted. Exact filters for independent integration:

- `daemon::scope::tests::passive_` (2): missing scope no effects and canonical alias/permissions preservation.
- `daemon::frame::tests::inspection_limit_rejects_length_before_reading_body`, `daemon::frame::tests::complete_envelope_must_fit_before_any_frame_is_written`, `daemon::frame::tests::hello_only_eof_is_distinct_from_a_partial_operation` (3).
- `daemon::observation::tests::` (3): actual blocked/failing initialization and cleanup; reset/read gate and retired sink; kernel-peer wrong handshake without business telemetry.
- `service::observation::tests::` (3): original IO cause before flattening; nested unwind plus write uncertainty; bounded ordered stages and terminal knowledge.
- `service::context::observation_tests::` (2): same-read declaration/effective snapshots immutable across later file edit; actual participant/KV provenance and withholding.
- `db::post_commit::tests::` (6): surviving savepoint facts; outer rollback exclusion; count bound; rollback budget restoration; independent encoded-byte bound; real reset rows excluded from old-era facts.

Existing `daemon::server::tests` supplies real queued generation/deadline rejection and admission/stop/panic/lease cleanup cases. Integration owns its new `daemon::observation_acceptance_tests` (7) and CLI leaf. Foundation filters and source releases remain in their original Tasks; do not treat the previous baseline's passed checks as evidence for these changed inputs. Capture an actual coherent fixed candidate after diagnostics release, then return concrete compiler/test diagnostics to the responsible source owner.

Configuration publication is selective/bounded; generic loader consumption has an honest empty actually-read subset, launch plans certify only their recorded selected fields, and full host census remains false. Explicit participant Resolve/Record covers observed reads, not a universal background participant census. Terminal declarations stay withheld without establishing builtin provenance. Reset-created rows are deliberately skipped until actual epoch publication rather than fabricated into either era. No cross-owner atomic snapshot, historical live-source reconstruction, universal receipt readback, automatic report retention guarantee, installed support or native Windows/macOS execution is claimed.

## Completion and receiving owner

Updated module guides: [`daemon`](../../../src/daemon/AGENTS.md), [`service`](../../../src/service/AGENTS.md), [`config`](../../../src/config/AGENTS.md), [`db`](../../../src/db/AGENTS.md), [`runtime`](../../../src/runtime/AGENTS.md) and [`messaging`](../../../src/messaging/AGENTS.md). They route future source readers to native policies and the exact lifecycle/producer/transaction constraints. Existing shared dirty work is preserved; no reset/stash/discard/commit/push or default/live scope/program was used.

All composition writes are released at this handoff; no process, terminal, service, endpoint, file lock, disposable fixture root or executable resource is retained. Main/CLI rights remain with diagnostics. Next action belongs to diagnostics/controller and independent native integration: complete CLI offering, freeze the coherent source, execute owned checks and route any concrete failures for a bounded correction. Backend/adapter/Web acceptance and Delivery closure remain with their assigned owners.
