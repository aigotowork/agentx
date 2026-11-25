# Daemon lifecycle and IPC

Start with [service contracts](../service/AGENTS.md), [lifecycle design](../../.docs/architecture/daemon.md) and [direct-call Task](../../.controller/direct-calls/task.md). Exact Rust declarations own the protocol; never maintain a second operation schema or direct-database fallback.

| Concern | Source and invariant |
| --- | --- |
| Canonical data scope and singleton | `scope.rs`; lock before application construction, retain through application drop; never unlink lifetime lock |
| Native local transports | `platform.rs` (Unix) and `windows.rs`; authenticated kernel peer PID/incarnation, private local endpoint, bounded cancellation |
| Frame encoding | `frame.rs`; native serde types, 8 MiB bound and absolute deadlines |
| Readiness and compatibility | `wire.rs`, `transport.rs`; one Call/Result with canonical scope, optional instance and generation, and per-operation recognition; version and build are diagnostic |
| Diagnostic build identity | Root [`build.rs`](../../build.rs) tracks source/assets/build inputs and emits a fresh UUID v4; `wire.rs` validates at compile time and reads loaded code, with no runtime image I/O/cache |
| Full artifact audit | Integration/package SHA-256 receipts and cfg(test) wire helpers; exact artifact bytes are independent of runtime compatibility and peer authority |
| Admission and dispatch | `server.rs`; scoped admission owns listener/32 workers, main dispatcher owns Application/16-job receiver, revalidate queued generation/deadline |
| Readiness events and cleanup | `platform.rs`/`windows.rs` wait on socket/pending-pipe events with unchanged 5 ms control tick; scoped stop/drop guards close listener and join workers on normal/error/panic exit |
| Caller binding | `../service/client.rs`; explicit invocation, retained generation and stable caller request IDs; never retry an uncertain effect |
| Explicit native recovery | `lifecycle.rs` and `lifecycle_tests.rs`; direct status and serialized graceful restart; `legacy.rs` can send only the original Shutdown on a verified epoch 3/4 connection |
| Persistence generation | Application/SQLite own durable key; `DatabaseGeneration` is a fail-closed in-memory mirror, never a lifecycle file |
| Work returned to hosts | [Relay model](../../.docs/architecture/relay.md); native responses carry control effects/frozen wake hints, actual host runs them after dispatch |
| Independent acceptance | `acceptance_tests.rs`; integration owns this leaf and all broad Cargo/build execution |
| Passive observation composition | `observation.rs`; lease-scoped native stores, read-only existing-scope lookup, finite authenticated envelopes and separate <=2-worker/1-second lane, independent of Application/SQLite and the ordinary 32-worker/16-job lane |
| Cloud messaging IPC | [Cloud IPC task](../../.controller/cloud-daemon/ipc.task.md); Linux scope-lease endpoint and bounded native frames, with HTTPS and exact hosted process fencing in the cloud client module |
| Observation cases | `observation_tests.rs`, scope/frame cases and the integration-owned `observation_acceptance_tests.rs`; see the [composition handoff](../../.controller/observability/implementation-tasks/composition.md) for exact authored filters and execution limits |
| Shared test fixture | `test_support.rs`; integration-owned real daemon/application helper and client shutdown helper compile only under cfg(test); production retains the shutdown envelope/handler |

Only the daemon entry constructs production persistence. Domain transactions and Provider effects remain in their owning services. Shutdown stops admission and drains the current application call; worker deadlines bound cleanup. Tests use disposable directories and exact source-built executables. Native Windows/macOS runtime evidence is distinct from source implementation; see integration acceptance before making platform support claims.

For restart and startup, see [operator usage](../../.docs/architecture/daemon.md#operator-status-and-recovery). Status uses a passive selected endpoint. Restart retains the startup lock across verified same-instance Shutdown, owner lock release and startup. Epoch 3/4 migration uses the same graceful shutdown path on every platform; failures do not permit force killing, lock deletion or a replacement.

Observation starts after acquiring ScopeLease and before Application initialization. Process-only Runtime evidence can be available while initialization is blocked; inspection results do not depend on the ordinary listener. On every exit, Application drops, ordinary admission/workers finish, the observer listener closes and its bounded workers join before lease release. A failed observer cannot fail or cancel business work. Passive resolution never calls `prepare`, repairs Unix permissions/Windows DACLs, creates `.daemon`, acquires/starts a daemon or reads managed SQLite.

Application remains the durable generation authority. `DatabaseGeneration` has a weak composition callback: withdraw journal/config availability before replacing the mirror, then let the lifecycle tick publish the exact committed epoch and fresh fixed-stream producer handles. The nonblocking adapter read gate plus generation rechecks yields a same-owner-era copy or explicit unavailable/changed; it promises no cross-store atomic snapshot. No observation publication lock spans Application, SQLite, I/O or response serialization. Late requests and prepared effects keep their original epoch; no old fact is redirected into a new stream.

Queue publication wraps only native nonblocking `try_send`; dequeue/dispatch pass the registry's short gate before validation. Execution, worker pin, reply wait and actual reply write are independent. An empty connection creates no request; a truncated operation remains a read error. Worker pin release follows the actual last write attempt, including the bounded failure envelope after wait abandonment. Do not turn a missing reply into cancellation, retry or a successful domain commit.

The native inspection frame budget is 64 KiB input/512 KiB complete output. Event leaves reserve 128 bytes for the fixed outer envelope, plus the measured encoded description. `DaemonChanged` is checked before every owner leaf, including unstamped Head/Describe. Origin-local failures retain structured stage/kind/raw OS code before service flattening; a missing failure timestamp establishes no time/order/age. Clock failure itself is ClockUnavailable with no fabricated timestamp. Windows read-only DACL/pipe and unsupported-platform branches remain source coverage until separately executed.

For compatibility maintenance, follow [direct calls](../../.controller/direct-calls/AGENTS.md). Add operations to the owned service declarations; reject unsupported operation tags separately from invalid inputs. Use the published compatibility module only during explicit restart. The earlier [epoch-based Task](../../.controller/domain-daemon/protocol-compatibility.md) is historical.


## Passive observation admission

`observation.rs::InspectionWorkers` owns retained inspection threads and reaps completed handles in `try_spawn`, after socket accept and immediately before the two-worker capacity decision. Pre-wait housekeeping also reaps, but cannot establish current capacity after a wait. Live response tails still occupy slots: true saturation drops the new connection without queueing or spawning a third worker, and worker deadlines bound shutdown joining. Receiving Result does not prove the server thread has returned.

`observation_tests.rs::inspection_admission_*` deterministically covers completion between housekeeping/admission, genuine active capacity, recovery and joining. The Linux acceptance fixture retries only write/read-stage I/O capacity closes with ECONNRESET/ECONNABORTED/EPIPE under a five-second retry window (an in-flight native attempt retains its one-second bound); it returns all owner responses and other failures unchanged. Its `passive_read_retry_is_bounded_and_does_not_hide_authority_failures` protects that distinction. Production `inspect_existing` retains its single-attempt one-second contract. [Repair Task](../../.controller/source-hygiene-diagnosis/repair.task.md) distinguishes authored coverage from Controller execution.


The authorized repair's [completed source outcome and independent evidence](../../.controller/source-hygiene-diagnosis/repair.task.md#completed-source-outcome-and-final-evidence) records15/15 exact native cases,1/1 actual CLI case, deterministic old-order discrimination and real transport capacity checks. This Linux local source-built result does not establish installed/live adoption.

The writer runs bounded reminder maintenance after each ordinary request and on idle timeout. The read lane does not drive timers. Keep derived-index backfills idle-only and preserve the reminder pass budgets; see [Events scheduling](../events/reminders.rs).
