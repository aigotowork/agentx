---
schema: task
status: completed
agent: Configuration provenance and invocation-context designer
---

# Configuration provenance and invocation-context designer

## Goal and professional authority

Define implementable structures and contracts that explain which configuration and environment facts applied to each daemon/client invocation and why two clients can behave differently.

Read the [team contract](../AGENTS.md), [Delivery](../delivery.md), [proposal](../../../.docs/proposals/agentx-observability.md) and [actual bindings](../controller.md#runtime-bindings). Apply: Domain/data/state modeling and source-grounded configuration contract design; distinguish configuration claims from authenticated authority. Current source is evidence; planned contracts must be labeled as design targets.

## Inputs

`src/config.rs`, `src/paths.rs`, `src/service/{context,client,queries}.rs`, scoped configuration store/service offerings, current environment/identity contracts and the architecture owner's shared vocabulary.

## Required result

1. Map actual scoped-file/default/environment/parsed-argument resolution and per-field exceptions from current source. Distinguish daemon startup environment, current scope declarations, invocation-captured environment, effective resolved values and actual launched-host facts. Do not impose one fabricated precedence chain.

2. Define typed configuration field identifiers, values/provenance/validation states, effective snapshot identity, subject/scope, capture/resolution revisions, comparisons and missing/redacted/unobserved meaning. Include concrete field/type/units/cardinality and reference lifetime obligations.

3. Specify native queries/report or capture capabilities for effective snapshots, provenance and bounded comparison, with inputs/results/errors, caller authority, failure and compatibility. Explain file edits after capture, partial validation, invalid overwritten values, restart/reset/generation and eviction.

4. Set capture/resolution/storage owners and retention boundaries. Only diagnostic disclosure approved by field policy may leave the owner; arbitrary full environment, credentials, actor capabilities and invocation authority must not become a generic management dump or new identity source. Preserve explainability after redaction without unsafe secret fingerprints.

5. Coordinate snapshot references and event provenance with architecture/runtime/events. A process's captured environment must not masquerade as continuous observation of that process. Define what implementation must retain at the actual resolution point to explain historic requests.

6. Derive focused scenarios for two clients sharing a daemon, different cwd/environment, scope aliases, config changes/invalid values and redacted differences; identify source insertion/consumer homes and downstream implementation requirements.

## Write and resource boundary

Own `.docs/observability/configuration.md` and this original Task. Coordinate shared-file transfers explicitly. Root navigation, roadmap and proposal/Delivery state remain controller-owned. Read-only current source inspection is authorized. No production source/dependency/package changes, no build/test/runtime probes, no nested agents, and no live/default scope access.

## Handoff and completion

Architecture consumes snapshot identity and shared references; runtime/events consume effective-context references; integration consumes comparison/explanation contracts and cases.

Release usable contracts early when they unblock a real consumer. Normal progress stays here; message only actionable contracts, concrete questions/findings or completion through AgentX. The final handoff must identify authoritative artifacts, resolved versus deferred decisions, scenario evidence, downstream obligations and released write/resources. Design completion never claims executable support.

## Working record

2026-11-07: Accepted the bounded configuration/context design ownership as Paseo agent `5989bbeb-55da-4eff-9220-c264a9f49792`; writes are limited to this Task and `.docs/observability/configuration.md`. No executable/runtime resource is claimed. The existing file Task is the execution authority; the Tasks integration also suggested `UNS-3` because no folder project or matching board item existed.

Method: source-grounded domain/data/state modeling. First separate scope selection, process startup inputs, scoped declarations, invocation claims, actual effective resolution and launched-host evidence. Derive identities and capture boundaries from two-client and file-change scenarios, then publish typed snapshot/reference contracts to runtime, architecture, events and integration. Record state/retention/disclosure rules with the design and retain only consequential decisions here. Design-only authority excludes source skeletons, builds, tests, live scopes and nested delegation.

Source findings: `src/config.rs::{FIELDS,AgentxConfig::load_for_scope_with_errors,load_from_sources_at,env_value}` supplies per-field environment/file/default rules, relay file-only exceptions, empty-value rules, normalization and per-field fallback. `src/config/file.rs::get_for_scope` exposes selected raw declaration/source rather than validated effective values. `src/paths.rs::resolve_agentx_dir_from_env` resolves scope input before `src/daemon/scope.rs::DaemonScope::prepare` canonicalizes it. `src/service/context.rs::{InvocationContext,CallerContext,RequestContext}` distinguishes client-supplied configuration claims and transport-observed authority; `src/service/client.rs::Client` retains the invocation and generation, and `with_invocation` changes claims without reacquisition. These contracts rule out a universal precedence chain and a globally authoritative daemon configuration snapshot.

Published [configuration design](../../../.docs/observability/configuration.md). The source map covers scoped file/default/environment/parsed-argument exceptions, raw `get` versus effective resolution, startup scope, actual server registration loads, launch environment regimes/overlays, participant overrides and native Provider boundaries. The [capture references](../../../.docs/observability/configuration.md#capture-and-snapshot-references) were released early to runtime/events/architecture/integration; runtime supplied its client invocation/attempt IDs and later exact `AcceptedPeer` connection/process evidence. The [reader/facade](../../../.docs/observability/configuration.md#retained-lookup-and-comparison) now supplies exact bounded read/report errors and comparison branches, with no fresh resolver on the passive inspection lane.

Consequential selections: immutable snapshots per actual source/use rather than a global effective configuration; same-pass resolver trace rather than later reconstruction; random read receipts rather than file/secret content hashes; fixed field disclosure with hidden equality always unknown; volatile FIFO retention rather than a persistent telemetry store/watcher; exact scope/daemon/generation fences rather than timestamp/name/PID joins. These choices preserve current native rules and make missing evidence explicit. The retained-ref epoch describes retention authority, not authority or time of a client report's source.

Peer reconciliation: runtime accepted `InvocationCaptureRef` with shared clone-safe `u64` revision and multiple per-attempt snapshots; configuration imports `AcceptedPeer` rather than defining a competing process identity. Events consumes exact `ConfigObservationEvent` without values and does not pin snapshots. Architecture's overflow finding was substantive: a same-epoch collector restart could alias sequence-only refs. The design now preserves its allocator/frontier across such replacement and stays unavailable on overflow until reset/restart produces a genuinely new `ObservationEpoch`; no nonce was added. Integration's missing facade/peer/allocator findings are resolved by exact `ConfigurationRequest/Response/Error`, alias and concrete `InvocationCaptureHandle` APIs. `StartupInputs` is explicitly present in the stage enum.

State/disclosure decisions and focused cases are authoritative in the design's [disclosure](../../../.docs/observability/configuration.md#disclosure-policy), [maintenance](../../../.docs/observability/configuration.md#state-maintenance-and-boundedness) and [scenario](../../../.docs/observability/configuration.md#focused-design-scenarios-and-implementation-obligations) sections. A reset racing a read returns a same-epoch frozen batch or explicit mismatch; old prepared publication is checked against its expected epoch under admission lock. Retired-ref lookup proves no-longer-retained but does not promise an eviction cause without retained removal-event evidence; no unbounded tombstone ledger is selected.

Recovery: an explicit provider high-demand error was reported by the controller during this assignment. Restored team/Task/materialized design/shared contracts and resumed the existing draft; no research restart, new agent or mechanism was introduced. All work remains design-only.

Final source/consumer reconciliation: [snapshot body](../../../.docs/observability/configuration.md#snapshot-subjects-and-provenance) carries explicit `resolution_scope`, separate from a report's retention epoch; scoped comparison rejects unestablished/different source scopes. [Comparison basis](../../../.docs/observability/configuration.md#retained-lookup-and-comparison) retains stage and each operand's purpose, preserving actual use versus later diagnostic reevaluation. Field validation fixes seconds/boolean/path/categorical value variants before report admission. These close identity/meaning holes without introducing a collector, durable telemetry or another authority source.

Completed handoff: exact planned configuration contract is released at [configuration.md](../../../.docs/observability/configuration.md), particularly [references/events](../../../.docs/observability/configuration.md#capture-and-snapshot-references), [capture/publication](../../../.docs/observability/configuration.md#capture-and-publication-operations), [lookup/comparison/facade](../../../.docs/observability/configuration.md#retained-lookup-and-comparison), [disclosure](../../../.docs/observability/configuration.md#disclosure-policy), [lifetime/bounds](../../../.docs/observability/configuration.md#state-maintenance-and-boundedness) and [focused cases/insertion obligations](../../../.docs/observability/configuration.md#focused-design-scenarios-and-implementation-obligations). Architecture/runtime/events/integration received actionable releases at their real Paseo bindings. No unresolved configuration-owned contract decision remains; domain consumers retain their own integration/acceptance work.

Evidence scope: read-only native source inspection and source-grounded design walkthroughs of same/different scope, per-field precedence/fallback/empty/invalid/overwritten cases, hidden equality, actual-use versus reevaluation, host plans/reports, participant incarnation, reset/restart and capture/lookup pressure. This owner ran no compiler, executable tests, document-check script, platform probe, packaged tool, live/default scope, service, package or dependency mutation. Independent integration owns the necessary document/contract checks and Delivery acceptance. These results establish design readiness, not shipped observation support.

Deferred by explicit boundary: executable instrumentation/native serde promotion, source tracing hooks and test execution belong to downstream implementation Tasks; durable history, file watchers, arbitrary environment/process inventory and live Web hosting are not selected mechanisms. Actual native Provider/domain behavior remains under its current owners. All writes to this Task and `configuration.md` are now released; no child process, build, database, endpoint, lock or temporary runtime resource is retained.

