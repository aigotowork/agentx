---
schema: task
status: completed
agent: Configuration evidence implementation owner
---

# Configuration capture, provenance and comparison

## Authority and entry

Read the root `AGENTS.md`, the [observability team entry](../AGENTS.md), [implementation Delivery](../implementation.md), [feature entry](../../../.docs/observability/README.md), [engineering map](../../../.docs/observability/engineering.md) and the specific contracts below. The user has authorized the implementation Delivery. Follow the [current implementation team contract](../implementation-team.md) and [actual assignments](../controller.md#implementation-assignments); this Task becomes active only under its assigned owner. Preserve the completed daemon baseline; execution uses explicitly disposable scopes and does not include installed/live cutover.

## Inputs and result contract

[Configuration design](../../../.docs/observability/configuration.md), [shared evidence](../../../.docs/observability/contracts.md#evidence-and-time), OBS-04/06/08/09/10 and [consumer disclosure](../../../.docs/observability/consumer-boundary.md#bootstrap-and-disclosure-decisions).

Implement immutable client capture and server resolution snapshots, field-policy disclosure, provenance and exact lookup/compare operations. Explain two client requests using what their actual resolution points retained. Distinguish scoped declarations, invocation claims, selected candidates, validation/normalization/fallback and actual launch observations.

## Write and resource boundary

Own `src/observability/{configuration.rs,configuration_store.rs}` and named tests. Own corresponding configuration design promotion and native guide sections. Existing `config.rs`, `config/file.rs`, `paths.rs`, launch/service/domain load sites remain composition-owned insertion locations; describe each required safe callback and source contract.

## Required work and decisions

Consume the actual loader's per-field precedence and exceptional rules; do not create a second settings resolver in telemetry. Retain only the owner's safe projected inputs and outcomes at the actual capture/resolution boundary. Implement the exact finite configuration facade, accepted-peer alias and clone-safe `InvocationCaptureHandle`; expected-epoch validation and insertion share one admission lock. Implement bounded retention, late/duplicate reporting, eviction, epoch checks and comparison branches exactly as selected. Same-epoch collector replacement preserves allocator/frontier; overflow requires a genuinely new epoch before capture resumes. Hidden fields cannot gain equality through fingerprints, raw values or unsafe candidate diagnostics.

## Authored cases and acceptance

Author same scope/different environments; file edit between capture and later read; invalid environment over valid file; file-only relay fields; empty-value exceptions; invalid overwritten candidates; normalized/fallback values; field-level withheld/unknown comparison; old-era/evicted refs and pressure. Keep new-resolution comparison distinct from historical evidence.

The [Delivery](../implementation.md) owns formal stage dependencies. Native/domain owners author cases; the assigned independent integration owner executes the necessary checks. Keep source analysis, authored fixtures, executed evidence and installed support distinct. Do not rerun a passed check without a changed relevant input or explicit requirement.

## Handoff and completion

Release native refs, capture/lookup/compare implementations, producer callback contracts and authored cases to runtime/events/composition/native integration. Composition alone inserts callbacks into actual loaders; identify the exact candidate and effective-result points so a reread cannot masquerade as an earlier snapshot.

Maintain this original Task's status and source/evidence basis. Replace promoted design declarations with native links, update the owning guide, identify remaining limitations, and release exact writes/processes/endpoints/roots before completion. No installed/live release is implied.


## Implementation activation

The user authorized this implementation batch after accepted design. The actual binding and source/resource scope are recorded in the controller. Initial work: Prepare configuration public declarations, safe capture/provenance/store skeleton and cases; consume actual shared imports and publish exact callback requirements to composition. Maintain further source releases, real blockers and next action here. Downstream callable dependencies and independent execution remain required for acceptance.

## Working record

2026-11-07: Accepted the new implementation Task in the existing configuration professional session. The completed design Task remains closed. Reused the accepted configuration domain design and native resolver source; began promoting its exact wire declarations into `src/observability/configuration.rs` before behavior. Source boundaries remain configuration/configuration_store and named tests; composition is the sole writer of existing config/service/domain insertion sites. Native facade/context and runtime peer/identity imports are being supplied by their actual owners. No duplicate shared IDs, setting resolver or generic environment map will be introduced.

Initial work, completed by the native release below: materialize safe projector/clone-safe local capture and bounded epoch-checked store, then author scenario cases. Requested exact shared clock/native imports and loader trace callback coordination. Existing Cargo uuid has v4 but no serde feature; source-read receipts will use validated UUID text without editing dependencies. The board integration proposed UNS-11 for this new work; original file metadata remains execution authority. No build/formatter/test/probe has been run by this owner.

Native milestone: `configuration.rs` now owns exact serde refs/facade, safe field projector and strict report validation, immutable local capture with shared revision allocation, bounded serde byte counting and source-scope-aware typed comparison. It imports actual `context.rs` and Runtime's actual UUID/AcceptedPeer types. `configuration_store.rs` supplies `ConfigurationStore` (re-exported through the public configuration leaf), same-lock expected epoch/admission/sequence allocation, same-epoch allocator preservation, unknown-generation closure, FIFO payload/count/age limits, per-observed-peer report quotas, late duplicate-report handling, in-memory reader/facade behavior and out-of-lock nonblocking event sink publication. Existing config/service/source files remain untouched by this owner.

Callable release to composition: constructor `ConfigurationStore::new(DaemonRef,ObservationClock,Option<Arc<dyn ObservationSink>>)`, lifecycle `rotate_epoch`/`invalidate_generation`/`set_sink`, `handle` and `ConfigurationReader`/`ConfigurationRecorder`; `InvocationCaptureHandle` methods and actual-load `ConfigResolutionTrace`/`ConfigFieldTrace`/`project_effective_config` callbacks. Explicit current evaluation uses injected `ConfigResolutionSource` at the real scoped loader. Missing trace stays unknown rather than being inferred or re-read. Composition owns tracing before parser-error suppression, selection/empty/coercion/fallback and custom-preset classification.

Authored unexecuted cases: `configuration_tests.rs` uses the real explicit scoped loader for invalid env/empty/relay rules and file-edit evidence, plus safe shadowed-invalid traces, no arbitrary environment/credential leakage, hidden equality, local eviction/revision uniqueness, unknown source scope and allocator exhaustion. `configuration_store_tests.rs` covers immutable pair comparison, same-lock reset versus publication/read, FIFO/age/frontier, late/client-only provenance, report rejection, per-incarnation/unknown-incarnation quota boundaries, pressure/overflow and sink reentry outside the store lock. The release below supersedes this preparation state; no execution claimed.


## Native source release and receiving owners

Configuration-owned implementation is complete for its scoped native offering. Exact declarations and behavior live in [configuration.rs](../../../src/observability/configuration.rs) and private [configuration_store.rs](../../../src/observability/configuration_store.rs); existing config/service/paths/domain sources were not edited by this owner. Actual imports are the released [context.rs](../../../src/observability/context.rs), [runtime.rs](../../../src/observability/runtime.rs) and [events.rs](../../../src/observability/events.rs), with no duplicate shared identities or dependency changes. The public configuration leaf re-exports `ConfigurationStore`; its consumers do not import the private store.

| Consumer | Callable release and next condition |
| --- | --- |
| Runtime | Exact `InvocationCaptureRef`, `ConfigStage`, `ConfigCaptureResult`/`CaptureSkipReason` and `ConfigurationRequest`/`ConfigurationResponse`/`ConfigurationError`. All wire values use native serde/Clone/Debug; refs have Eq/Hash. Runtime retains references/dispositions without raw environment or launch maps. |
| Events | Exact `ConfigSnapshotRef` and `ConfigObservationEvent::{SnapshotPublished,CaptureSkipped,SnapshotsRetired}`. Publication retains original source origin/time; removal/skip uses the emitting boundary clock. Sink callbacks run after the store lock. Events never pin configuration payloads. |
| Composition | `ConfigurationStore::new` accepts actual `DaemonRef`, shared `ObservationClock` and optional sink; `rotate_epoch`, `invalidate_generation`, `set_sink` and `handle` implement lifecycle/facade. `ConfigurationReader`/`ConfigurationRecorder` expose only selected ports. The actual-load port is `ConfigResolutionSource`, with `ConfigResolutionTrace`/`ConfigFieldTrace` consumed by `project_effective_config`. Existing-source callbacks remain solely composition-owned. |
| Originating adapter and diagnosis | `InvocationCaptureHandle::{begin,revise,reference,capture_invocation_inputs,capture_current_resolution,capture_resolved,retained}` owns shared clone revision/local allocation. `CaptureCurrent` takes borrowed invocation, explicit scope, independently established `resolution_scope` and actual resolver port. `compare_local_snapshots` preserves original local bodies/purposes without acquiring a daemon. |
| Independent native integration | [configuration_tests.rs](../../../src/observability/configuration_tests.rs), nine source/capture cases, filter `observability::configuration::tests::`; [configuration_store_tests.rs](../../../src/observability/configuration_store_tests.rs), thirteen store/facade cases, filter `observability::configuration_store::tests::`. Freeze with the other foundation/composition releases, then execute the coherent candidate and route exact owned diagnostics. |

## Final method decisions and evidence limits

The same-pass source boundary is actual `AgentxConfig::load_for_scope_observed` plus composition's [config/observation.rs](../../../src/config/observation.rs). Authored source cases now consume its real trace and `NativeConfigResolutionSource`, including invalid selected environment over valid file, empty exceptions, relay file-only policy, parser-known overwritten invalidity, normalization, malformed source and a custom preset overriding a builtin name. File-edit capture uses actual original resolution plus explicit `capture_current_resolution` with `DiagnosticReevaluation`; the old snapshot and original receipt remain unchanged.

Expected epoch, sequence allocation and immutable insertion share one store lock. Reads clone a same-era pair there, then compare outside it; reset returns consistently stamped evidence, `EpochChanged` or `Busy`. Capture/read counters use checked arithmetic and never wrap. Nonblocking skip counters are lower-bound evidence only: if the counter update itself contends it can be absent, while the caller receives explicit `Skipped`. Old-era skips cannot increment a successor's counters. Exhaustion does not recover on same-epoch rotation. Composition constructs one store per daemon lifetime and must retain its allocator/frontier through clones rather than constructing a replacement in the same daemon/era.

Projection validates bounded source counts/steps before cloning them, excludes raw parser candidates and credential details, and preserves unknown evidence when tracing is missing. UUID-v4 source receipts use validated text with the existing dependency features. Source references must name the matching receipt/resource; participant `DomainRecord` projections use the native typed subject without an invented file receipt, while persistent dev-root KV provenance requires a `PersistentDevRoot` receipt. Hidden equality is always unknown, including a missing concealed field or an identical snapshot reference. Generic reports remain client claims bound to accepted transport peers; only the native host port can retain a validated Host subject.

The [configuration model](../../../.docs/observability/configuration.md) now links native declarations instead of duplicating Rust schemas, preserves semantic tables and documents [precise actual load-site instructions](../../../.docs/observability/configuration.md#native-load-site-handoff). The configuration section of [native guidance](../../../src/observability/AGENTS.md#configuration-evidence-maintenance) routes future owners to source, policies, cases and independent evidence.

No Cargo, Node, formatter, tests, build, package or runtime probes were executed by this owner. Source inspection and case authorship are the available evidence; compile/runtime/CLI/platform/installed support is not asserted. Composition still owns actual startup/client/launch/validated-host/participant hook coverage, consumed-field facts, passive authentication and lifecycle wiring. Native integration owns fixed-candidate execution, finite-envelope transport evidence and exact cleanup. Those assigned downstream conditions do not reopen this completed scoped offering.

## Released writes and resources

Released configuration-owned writes: `src/observability/{configuration.rs,configuration_store.rs,configuration_tests.rs,configuration_store_tests.rs}`, `.docs/observability/configuration.md`, the configuration-maintenance section of `src/observability/AGENTS.md`, and this original Task. No process, terminal, endpoint, database, disposable runtime root, compiler/formatter lease, filesystem lock or installed/live resource was created or retained. All unrelated dirty work is preserved. Further owned fixes require an exact diagnostic from the independent integration/receiving Task; no additional review cycle or speculative work is pending.


## Candidate-01 strict Clippy repair

Native integration transferred only the originally owned source/test files for diagnostics from `/tmp/agentx-obs-native-8hyqm9g2/observation-clippy-01-configuration.json` (36 diagnostics), with full log `/tmp/agentx-obs-native-8hyqm9g2/observation-clippy-01.log` and frozen 445-file input `/tmp/agentx-obs-native-8hyqm9g2/observation-candidate-01-source`. Task is active for this bounded repair. Current work: preserve MSRV 1.89, accepted finite inline native contracts and current behavior; correct nested conditionals and narrowly justify lints at the owning declarations. No frozen evidence, shared composition source, dependency or execution resource is modified. Next condition: source-only correction release to the same independent integrator, who alone formats/freezes/executes the changed candidate.


Repair source release: changed only `src/observability/configuration.rs`, `src/observability/configuration_store.rs` and this Task. Preserved the MSRV-compatible `AtomicU64::fetch_update` with a method-local deprecation allowance on `revise`; did not adopt the current compiler's newer replacement. Added a single named `result_large_err` allowance per owned leaf, with the exact rationale that finite era errors preserve requested/current native identities inline without allocating on nonblocking failure. Added declaration-local `large_enum_variant` allowances only on the five diagnosed bounded snapshot/request/response/comparison enums, avoiding shared facade/Box signature churn. Added item-local dead-code rationale on `ConfigEvidenceRef` and test-aware allowances on the released local historical lookup/comparison offering, which downstream diagnostics has not consumed yet. Rewrote the two diagnosed nested conditionals as equivalent Rust 1.89-supported let chains. No global lint disable, dependency change, behavioral result change or test mutation was introduced.

The 36-item diagnostic list and full-log excerpts were inspected against the actual current sources. This is a source correction, not a passing Clippy claim. Existing nine configuration and thirteen store authored cases remain applicable and unexecuted by this owner. Independent integration alone formats these two corrected files, freezes the changed derivation and executes necessary checks; candidate-01 frozen input/logs remain untouched. Bounded repair writes are released again, Task completed, no process/endpoint/root/compiler resource retained.


## Candidate-02 test initializer repair

Active bounded transfer: only `src/observability/configuration_tests.rs` fixture initialization plus this original Task state. Native integration reported `field_reassign_with_default` in its retained candidate-02 Clippy log. Preserve all four redaction fixture values and assertions with an equivalent struct initializer; no lint suppression or execution by owner. Frozen candidate-01/02 inputs and receipts remain untouched.


Candidate-02 correction released: only `src/observability/configuration_tests.rs` changed, replacing four assignments after default construction with the equivalent `AgentxConfig { relay_psk, relay_token, hints, terminal, ..AgentxConfig::default() }` initializer. Values and all scenario assertions are unchanged; no lint allowance added. Applicable case: `observability::configuration::tests::projector_drops_all_credentials_and_custom_terminal_even_for_same_ref`. No formatter/check/test/probe executed by owner; native integration owns formatting, changed derivation and execution. Original Task completed and bounded test writes/resources released again.
