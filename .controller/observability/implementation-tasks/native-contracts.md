---
schema: task
status: completed
agent: Native contract and engineering owner
---

# Native shared contracts and context authority

## Authority and entry

Read the root `AGENTS.md`, the [observability team entry](../AGENTS.md), [implementation Delivery](../implementation.md), [feature entry](../../../.docs/observability/README.md), [engineering map](../../../.docs/observability/engineering.md) and the specific contracts below. The user has authorized the implementation Delivery. Follow the [current implementation team contract](../implementation-team.md) and [actual assignments](../controller.md#implementation-assignments); this Task becomes active only under its assigned owner. Preserve the completed daemon baseline; execution uses explicitly disposable scopes and does not include installed/live cutover.

## Inputs and result contract

[Shared references/evidence](../../../.docs/observability/contracts.md#shared-references), [valid construction/failure](../../../.docs/observability/contracts.md#valid-construction-and-failure), [context model](../../../.docs/observability/model.md) and [engineering homes](../../../.docs/observability/engineering.md#support-and-structure-decision).

Promote `ScopeRef`, `DaemonRef`, `ObservationEpoch`, clock/unit wrappers and evidence meaning into native Rust. Preserve original field/absence/provenance semantics and reuse existing domain value refs. Establish the observability facade and context route so foundation owners can publish their own public offerings without inventing shared substitutes.

## Write and resource boundary

Own `src/observability/{mod.rs,context.rs,AGENTS.md}` and the additive module declaration in `src/main.rs` only. Release `main.rs` to composition immediately after that declaration. Shared design promotion is limited to the corresponding declaration blocks in `contracts.md` and route links in `engineering.md`; coordinate those regions with their design owner. Runtime/config/events public declarations belong to the foundation Tasks, not this owner.

## Required work and decisions

Materialize the shared boundary and only the structure needed for the selected implementation. Implement pure constructors and `ObservationValueError` validation without filesystem/transport I/O; ClockId construction uses the selected per-producer origin rather than an implicit process-global comparison. Publish the exact native imports and compile context to the three foundation owners. Document dependency direction and domain ownership in the module guide. Replace the promoted shared declaration block with a source link; do not retain two editable definitions or create executable behavior stubs that return scripted success.

## Authored cases and acceptance

Author shared reference equality cases covering alias canonicalization inputs, same scope/different daemon, same daemon/different generation and unrelated clocks. Cover unknown generation without an empty epoch, zero durations and rollover rejection. These declarations cannot themselves prove OS identity or lifecycle behavior.

The [Delivery](../implementation.md) owns formal stage dependencies. Native/domain owners author cases; the assigned independent integration owner executes the necessary checks. Keep source analysis, authored fixtures, executed evidence and installed support distinct. Do not rerun a passed check without a changed relevant input or explicit requirement.

## Handoff and completion

Release native shared refs, facade ownership and the exact build/source impact to all foundation owners and composition. Foundation Tasks become ready when these imports and the module entry are usable. Integration later executes required checks; this owner does not launch broad builds.

Maintain this original Task's status and source/evidence basis. Replace promoted design declarations with native links, update the owning guide, identify remaining limitations, and release exact writes/processes/endpoints/roots before completion. No installed/live release is implied.


## Implementation activation

The user authorized this implementation batch after accepted design. The actual binding and source/resource scope are recorded in the controller. Initial work: Promote the accepted shared model to native declarations and publish exact imports early; hand main declaration region to composition after release. Maintain further source releases, real blockers and next action here. Downstream callable dependencies and independent execution remain required for acceptance.

## Working record

- Restored actual root/team/implementation entries, original Task and current native ownership. This is a new implementation assignment; the prior architecture design Task remains completed. Existing dirty checkout was inspected as a preservation boundary and is not this Task's baseline. Board mirror: UNS-12; original Task metadata remains execution authority.
- Structure/materialization: created `src/observability/{mod.rs,context.rs,AGENTS.md}` with public contract modules and private stores. Added only `mod observability;` to `main.rs`, then explicitly released main to composition at binding `b56613b3-d267-4b49-996b-f80a3fbec937`. Foundation owners received actual shared imports and may re-export concrete constructor handles through their own public leaf.
- Implemented pure reference construction/serde validation, checked duration/deadline arithmetic, evidence variants and a retained cloneable `ObservationClock` with explicit per-producer origin. `now`/`at_instant` return validation errors rather than fabricate a time. No filesystem canonicalization, environment capture, DB/process/transport I/O or dependency changes occur in shared values.
- Shared fields retain selected names and derive Clone/Debug/serde/equality/hash; units additionally Copy/Ord. Serde validates native boundaries without relying on callers to remember a separate validator. The clock uses existing UUID v4 support with string encoding, no uuid serde feature needed.
- Authored nine inline cases under `observability::context::tests`; manually reviewed native values, validation and the scope/restart/reset/time/provenance cases against the selected contracts. Replaced the promoted shared declaration blocks in `contracts.md` with native links and added native guide/facade routes in `engineering.md`. These are source review and authored cases only; none has been executed.
- Native exports and retained clock API were released to all three foundation owners and composition. Independent native integration received the stable shared import/source release and exact test filter. Main was released immediately after its additive declaration; no subsequent main writes were made by this owner.

## Native implementation handoff

The actual implementation binding is `{serverId: srv_yDbm9zuqIMiM, agentId: 54397ed6-da22-4edc-a640-f8a8deb9e93c}`. This local assignment is complete; the [independent native integration Task](native-integration.md) owns execution and backend acceptance. It receives actual native values and authored cases, not a full-product support claim.

| Artifact | Released result and source impact |
| --- | --- |
| [Shared native values](../../../src/observability/context.rs) | Sole exact authority for scope/daemon/epoch/evidence, validation errors, serde boundary validation, checked time arithmetic and the retained cloneable `ObservationClock`. Constructors have no filesystem, database, transport or process-control effects. |
| [Facade](../../../src/observability/mod.rs) | Public crate-local `context`, `runtime`, `configuration` and `events`; private `registry`, `configuration_store` and `journal`. Each domain owner can re-export its concrete constructor through its own public leaf; sibling private-store imports remain excluded. |
| [Native guide](../../../src/observability/AGENTS.md) | Dependency direction, ownership, one daemon clock supplied by composition, exact epoch fences, disclosure and independent execution route. |
| [Executable declaration](../../../src/main.rs) | Only the additive `mod observability;` was made by this Task. Whole main write ownership was released to composition before its wiring began. |
| [Shared semantics](../../../.docs/observability/contracts.md#shared-references), [engineering route](../../../.docs/observability/engineering.md#support-and-structure-decision) | Native promotion replaces editable shared signature copies; semantic explanations and source links remain. |

### Decisions and source basis

| Decision | Basis, consequence and limit |
| --- | --- |
| Keep scope canonicalization and lease authentication outside value constructors | Existing [`ScopeLease`](../../../src/daemon/scope.rs) owns the actual instance and scope. `ScopeRef` validates an already resolved absolute input without I/O; identical canonical inputs compare equal, but these tests do not prove real alias resolution or OS identity. |
| Reuse the exact committed generation, independently of daemon incarnation | [`DatabaseGeneration::publish`](../../../src/daemon/scope.rs) already validates committed UUIDs; [`Application`](../../../src/service/application.rs) publishes the durable key. The wrapper copies that value, rejects missing/invalid values and cannot create or rotate durable generations. Process-only unknown generation remains separate evidence. |
| Retain and explicitly supply one clock per producer origin | `std::time::Instant` supports checked elapsed time; existing UUID v4 and serde dependencies provide string IDs and boundary validation without manifest changes or UUID serde features. Clones preserve the same origin; separate construction is a separate clock domain. Wall time remains optional and never establishes causal order. |
| Validate serde at ingress as well as pure constructors | Nested relative scopes, empty instances, invalid generation and invalid clock values are rejected during decoding. In-process fields remain native composition inputs; shape validation does not authenticate their source. |
| Keep cross-domain instrumentation in composition | The selected adapter publication gate fences all store rotations after authoritative generation publication. A scoped RAII dispatch observation context may carry frozen producer refs/clock/epoch, but grants no business authority. Postcommit projections retain their original fence rather than consulting a later dispatch context. Actual wiring and concurrency evidence remain composition/integration-owned. |

### Authored cases and execution boundary

The nine unexecuted test leaves are in [`context.rs`](../../../src/observability/context.rs), selected by `observability::context::tests`:

- `scope_identity_uses_supplied_canonical_root_without_resolving_aliases`
- `restart_and_reset_are_independent_identity_fences`
- `missing_generation_cannot_construct_a_dataset_epoch`
- `serde_rejects_invalid_nested_references_at_the_boundary`
- `clock_identity_requires_a_v4_uuid_in_native_and_serde_construction`
- `elapsed_time_uses_one_clock_even_if_wall_time_moves_backwards`
- `duration_and_deadline_arithmetic_reject_rollover_instead_of_wrapping`
- `retained_clock_clones_share_origin_but_new_clocks_do_not`
- `provenance_and_absence_states_survive_serialization_without_conflation`

No Cargo, Node, formatter, tests, probes, build/package operation or check script was executed by this owner. No process, endpoint, temporary scope, service, build resource or installed artifact was acquired. The module tree includes other owners' evolving implementations; shared release alone does not establish a coherent compiled candidate, lifecycle/IPC instrumentation, passive CLI, browser support or unexecuted-platform compatibility. Integration freezes a coherent candidate and returns exact diagnostics to the relevant owner.

All owned source/document writes are released. Composition receives the bounded next-write right for compatible facade declarations/exports required by this Delivery; shared semantic changes still need their actual architecture/domain decision owner. No write lock or permanent module role is retained. The next condition is independent execution on the combined released candidate, tracked by the receiving original Task rather than this completed local assignment.

## Clock-unavailable local failure reconciliation

Composition reported a concrete native incompatibility: [`LocalFailure.at`](../../../src/observability/runtime.rs) required an `ObservationTime`, while [`local_failure` and `inspect_existing`](../../../src/daemon/observation.rs) must return a failure even when the fallible shared clock cannot represent the time. The moving adapter used a maximum monotonic value as a fallback, which would fabricate evidence. Current work is a bounded shared semantic decision and owner handoff; no domain or composition source write is acquired.

Selected minimum correction: runtime owns an optional local-failure capture time plus a precise `ClockUnavailable` failure kind for the clock facility itself. Unavailable timestamp capture preserves any independently known original failure kind, stage and OS code; an operation whose own failure is the clock uses `ClockUnavailable` without a timestamp or OS code. No sentinel, new clock origin, inferred wall time, extra result wrapper or business failure reinterpretation is selected. Runtime owns declaration/validator/test and semantic-document updates; composition owns adapter mapping and removal of the fallback. Report start/end/span times remain required and must never be synthesized to admit an otherwise unrepresentable report.

The exact decision and native edit/test requirements were delivered to Runtime and Composition, with the affected input shape delivered to independent native integration. The bounded architecture response is complete and owns no further source writes. Actual correction/source release remains with the Runtime and Composition original Tasks, and executable acceptance stays with native integration. Raw `io::ErrorKind` may be classified at the adapter before conversion, retaining any OS code; a classification already lost must remain unknown/generic IO rather than be guessed from message text. No compiler, formatter, tests, probe, process, endpoint or build resource was acquired during this response.

## Reset projection fence disposition

On the user's continuation, inspected the concrete composition finding against [`without_dataset_observation`](../../../src/service/context.rs), the [`ResetConversation` insertion](../../../src/service/maintenance_store.rs), [`Application` generation publication](../../../src/service/application.rs), [`EventAppended` projection](../../../src/db/events.rs) and the authored [`reset_created_rows_are_not_published_under_the_admitted_old_epoch`](../../../src/db/post_commit.rs) case. The selected suppression is compatible with shared invariants: reset writes new-dataset rows before Application publishes its committed generation, so those rows cannot inherit the admitted request's old epoch. Instrumentation-only RAII removes and restores the context; it neither generates a new epoch nor changes reset authority/results. The outer request can retain old decode generation and actual generation-after chronology. Prepared effects retain their original fences/sinks.

This is an explicit selective collection gap, not a promise of complete reset-event projection, a numeric cursor gap or proof that no domain effect occurred. Actual execution remains native-integration-owned; source inspection and the existence of the authored case do not establish a pass. No shared native API correction, sibling write, recovery prompt, build/test/probe or resource was acquired. The composition owner had a provider high-demand error with its original Task still active; controller retains recovery and final source-release ownership. Shared native assignment remains completed and released.

## Candidate-01 strict Clippy correction

Independent native integration granted a bounded repair transfer for one owned diagnostic: `src/observability/context.rs:189:19`, `checked_add` unused in the production target. Exact receiving evidence is `/tmp/agentx-obs-native-8hyqm9g2/observation-clippy-01-native-contracts.json`; the preserved full log is `/tmp/agentx-obs-native-8hyqm9g2/observation-clippy-01.log`. The diagnostic refers to `MonotonicMillis::checked_add`, not an addition method on `DurationMillis`. Current work is to retain the accepted checked deadline arithmetic with a method-local, explained dead-code allowance; no artificial production call or shared-contract removal is justified. Only the current checkout's owned source and this Task are writable; frozen evidence stays untouched. Integration alone formats/freezes/verifies the corrected derivation.

Applied the method-local `#[allow(dead_code)]` with its rationale immediately above `MonotonicMillis::checked_add`; its implementation/signature, callers, serde and all time/error semantics are unchanged. Existing `observability::context::tests::duration_and_deadline_arithmetic_reject_rollover_instead_of_wrapping` remains the applicable authored case and was not rerun by this owner. No new test mirrors an attribute. Source inspection only; no Cargo/formatter/test/probe or check script ran. Both changed files (`context.rs` and this Task) are released to integration; no build/process/endpoint resource acquired and no frozen candidate/evidence file modified. The local correction is complete; strict Clippy acceptance awaits the independent executor's corrected derivation.
