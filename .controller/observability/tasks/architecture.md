---
schema: task
status: completed
agent: Architecture and engineering governance designer
---

# Architecture and engineering governance designer

## Goal and professional authority

Establish the shared semantic foundation and maintainable engineering map that permit the runtime, configuration and events designs to compose without duplicate authority.

Read the [team contract](../AGENTS.md), [Delivery](../delivery.md), [proposal](../../../.docs/proposals/agentx-observability.md) and [actual bindings](../controller.md#runtime-bindings). Apply: Engineering design, data/state modeling, domain-oriented structure and design-management maintenance. Read the relevant guides from the team entry before work. Current source is evidence; planned contracts must be labeled as design targets.

## Inputs

Current `.docs/model/`, `.docs/architecture/{daemon,contracts,principles,acceptance}.md`, `.docs/source-layout/README.md`, native `src/service`, `src/daemon`, domain offerings and the completed daemon Delivery.

## Required result

1. Publish an early source-grounded domain/context map and minimal shared vocabulary, then reconcile actual domain offerings. Distinguish observation identities from participant authority, client calls from persistent runtime hosts, declared configuration from effective captured facts, and volatile telemetry from durable domain facts.

2. Define concrete typed shared identifiers/references, observation validity/freshness/unknown meaning, time/units, source provenance and incarnation/generation fences. Reuse existing native IDs by reference. Own shared types only when meaning and lifetime truly match; leave offered/required operations in their domain owner.

3. Resolve cross-domain contract incompatibilities with their professional owners. Shared contract catalog must specify owner, purpose, fields/types/units/cardinality, input/result/error and actual read/write/effect/compatibility obligations, using implementable Rust-like declarations labeled as planned.

4. Design planned source/module/directory boundaries, composition/dependency direction, native/transport/storage representations, change-impact rules and context entry/maintenance ownership. Specify where declarations will be promoted into native source and obsolete design copies removed.

5. Record proportional mechanism decisions and rejected alternatives, including cross-scope discovery, read-only diagnosis versus controlled existing management operations, observation under dispatcher congestion, and retention authority. Do not choose a framework merely to fill a technology list.

6. Keep README and the artifact catalog navigable; current versus selected target support must be explicit. Link the original Proposal, Delivery, owner docs and intended implementation contracts.

## Write and resource boundary

Own `.docs/observability/README.md`, `.docs/observability/model.md`, `.docs/observability/contracts.md`, `.docs/observability/engineering.md`, `.docs/observability/.xmeta.yaml`, `a focused observability route in .docs/model/README.md` and this original Task. Coordinate shared-file transfers explicitly. Root navigation, roadmap and proposal/Delivery state remain controller-owned. Read-only current source inspection is authorized. No production source/dependency/package changes, no build/test/runtime probes, no nested agents, and no live/default scope access.

## Handoff and completion

Domain owners consume shared references; integration consumes the unified model, engineering map and selected cross-domain decisions. Release early vocabulary and final compatible design through assigned AgentX participants, then respond only to concrete owned corrections.

Release usable contracts early when they unblock a real consumer. Normal progress stays here; message only actionable contracts, concrete questions/findings or completion through AgentX. The final handoff must identify authoritative artifacts, resolved versus deferred decisions, scenario evidence, downstream obligations and released write/resources. Design completion never claims executable support.

## Working record

### Source and method basis

- Read the actual root/team entries, Delivery, Proposal, original Task and actual Paseo bindings. Applied engineering design (contracts before representations; consumer-owned required ports), data/state modeling (separate identities, unknown effects, lifetimes) and design management (one authority, current-versus-target views). Paseo was used only for communication during this historical execution; no nested delegation or product execution.
- Current native basis: `src/service/context.rs::{RequestId,InvocationContext,CallerContext,CallerProcess,RequestContext}`, `src/daemon/wire.rs::DaemonInfo`, `src/daemon/server.rs::{run,connection,enqueue,dispatch_jobs}`, `src/daemon/mod.rs::DaemonError`, `src/agents/attachments.rs::{ParticipantRef,ActivationTarget,ProviderSessionKey}` and source/module guides.
- Confirmed: one request per Hello connection; bounded 32-worker admission runs independently of the sole 16-job application dispatcher; process evidence is transport-observed and optional incarnation is meaningful. Logical `RequestId` is not an attempt/connection identity. Restart, database replacement and participant/attachment changes are independent fences.
- Consequential initial decision: share scope/daemon/database observation epoch plus time/evidence semantics; retain attempts/connections/hosts, configuration capture and stream/cursor identities with their actual domain owners. Observation does not own business effect classification or durable retention.
- Friction retained for the controller: `.docs/model/README.md` still contains an older “Today, AgentxDb is opened by multiple ...” narrative, although the completed daemon Delivery and current service/storage guides are the implementation baseline. This Task owns only a focused observability route; it must not silently rewrite historical baseline sections.
- Board integration suggested `UNS-4`; the original file remains execution authority. No project was bound to this folder, and the suggestion does not block work.

### Early contract release and decisions

- Published `.docs/observability/{README,model,contracts,engineering}.md`, the local catalog and focused `.docs/model/README.md` route. Released exact `ScopeRef`, `DaemonRef`, `ObservationEpoch`, process clock/units and evidence semantics to runtime/configuration/events; published native `src/observability/` homes and passive `daemon/observation.rs` adapter home to integration through assigned AgentX participants.
- Chose two independent identity levels: daemon/process evidence can exist before committed DB generation; dataset-bound refs require the complete shared epoch. Existing logical request/domain reconciliation IDs remain distinct from runtime attempt/connection IDs, configuration capture/snapshot refs and event stream nonce/sequence.
- Reconciled events' initial duplicate `ObservationEpoch`/old `src/observation` home with its owner: the owner is replacing them with imported shared epoch and owned `ObservationStreamId { epoch, nonce }` in `src/observability/events.rs`. Integration was told to consume the corrected release. No sibling edits.
- Selected direct bounded retained-config/event reads on the same passive diagnostic lane; no subscriptions/waits or Application/SQLite work occupy it. Runtime owns exact lane limits and aggregate transport composition. New config resolutions retain their real domain load-site authority rather than masquerading as cheap snapshot reads.
- Added explicit shared construction/error semantics, one process clock per daemon composition, and reset/publication obligations. No global atomic cross-domain snapshot, telemetry generation authority, machine-global discovery, generic mutation or new durable telemetry mechanism is selected.
- Controller dispositioned the baseline documentation drift in its owned feedback and corrected only the stale baseline paragraphs. This resolves the recorded friction without expanding this Task's write scope.
- Consumed actual planned implementation Delivery/10 Tasks and native/composition Tasks; the single composition writer owns shared insertion files while foundation owners retain their native modules. Added actual downstream artifacts to the local catalog; no implementation started.
- Substantive source-grounded contract correction: Events' original `OutputEvidenceRef.output_id` plus a separate `through` field reconstructed native `OutputToken { id, participant, through }` (`messaging/service.rs:54`, `service_store.rs` confirmation validation). Raised with the Events owner, who replaced it with fresh observation-only ID and replaced complete delivery receipts with boundary/disposition projection. Shared contracts now require disclosure safety across field/record joins. Existing mutation/domain authority remains unchanged. This is a design finding and correction, not an executed exploit/probe.
- Platform high-demand errors interrupted Runtime and Configuration before their full offerings were released. One targeted resume prompt was sent to each existing bound owner and the controller was informed; no new agents or ownership transfer. Runtime's original Task remains active and `runtime.md` is still unpublished at that interruption; Configuration has released refs/source/values but not its complete operations/lifetimes. Shared integration remains incomplete until those actual owner releases exist.
- Both original owners recovered and released consumable runtime/configuration operations. Consumed runtime identity, passive lane, stage/result/reply separation, metrics, bounded registry/report contracts and config recorder/reader, disclosure, FIFO retirement and same-epoch read linearization. Added exact operation/error anchors and budgets to the shared offering catalog; registered runtime's real artifact.
- Final concrete corrections requested directly from owners: per-scope Unix inspection endpoint discriminator, adapter aggregate versus runtime leaf declaration homes, `effective_deadline` optional-evidence branch consistency, daemon qualification of events' local request sequence, and config allocator non-reuse across same-epoch collector replacement. Complete routed-response byte bounds include outer enum/Result overhead in engineering.
- Consumed Configuration's corrected exact facade/peer alias/capture-handle release: explicit expected epoch is checked under insertion lock, StartupInputs is a distinct stage, same-epoch replacement preserves allocator/frontier and overflow waits for a real new shared epoch. Events finalized daemon-qualified filters and an authenticated-intake wrapper preserving historical report claims; owns no runtime resource or remaining write.
- Final aggregate rejection hole raised with Runtime: old expected daemon plus Configuration Head(None)/Events Describe lacks an old leaf dataset epoch; adapter must return an exact route-neutral daemon-changed result rather than invent a leaf EpochChanged or return a different response family. Waiting only for this owned correction/final release and integration's existing bounded evidence.
- Runtime applied the route-neutral `InspectionResponse::DaemonChanged { expected, actual }` before leaf routing and published concrete `RuntimeRecorder`/`RuntimeReporter`, AcceptedPeer-bound reader and safe retained-report list/detail operations. Endpoint, aggregate home, deadline-evidence and local-stage declarations are now consistent with the selected shared boundaries. Offering index links exact owner declarations instead of copying a drifting method signature.
- Consumed Configuration's final source-versus-retention distinction: body `resolution_scope`, explicit `ScopeUnestablished`, and comparison stage/purpose basis. Shared decisions now require source scope/purpose in addition to destination epoch before comparing reports. Configuration and Events released their writes with no unresolved owner decisions.

### Final handoff and evidence

- Authoritative output: [feature entry](../../../.docs/observability/README.md), [context/domain model](../../../.docs/observability/model.md#context-and-authority-map), [shared native declarations and constructors](../../../.docs/observability/contracts.md#shared-references), [released offering index](../../../.docs/observability/contracts.md#released-offering-index), [resolved boundaries](../../../.docs/observability/contracts.md#reconciled-boundary-decisions), [engineering/dependency/context map](../../../.docs/observability/engineering.md), local [artifact catalog](../../../.docs/observability/.xmeta.yaml), and the focused observability row in [model entry](../../../.docs/model/README.md). Domain methods/types/operations remain in runtime/configuration/events and are linked rather than recopied.
- Resolved: source/epoch/stream/native-home conflicts, passive congested-dispatch diagnosis, exact adapter and peer context, safe report provenance, token reconstruction, request-filter qualification, no same-epoch sequence reuse, complete-frame budgets and source-scope/purpose comparison. Consequential alternatives and reopen conditions are in model.md#decisions-and-proportionality. No architecture-owned semantic decision remains unresolved.
- Evidence: source-linked ownership/producer/consumer inspection plus manual contract consumption of the released runtime/configuration/events designs and integration's [OBS-01..10 walkthroughs](../../../.docs/observability/scenarios.md#required-supply-and-contract-walkthroughs). The [independent design acceptance](../../../.docs/observability/acceptance.md) owns exact scenario dispositions and its single necessary document/contract check. Architecture ran no automated check, compiler, test, runtime/platform probe, package operation or live-scope command; no executable support is claimed.
- Downstream: promote planned declaration blocks into their assigned native homes and replace copies with source links; enforce actual bounded synchronization and publication order; execute the [planned implementation Delivery](../implementation.md) in its dependency order, preserving sole shared-file composition ownership and independent acceptance. Browser transport/assets and runtime support remain planned. Global discovery, durable telemetry, arbitrary process/environment inspection, new business mutation and installed/live cutover remain outside this design.
- Released all owned documentation/Task writes to receiving integration/controller. No source writes, active locks, subprocesses, endpoints, temporary roots or product resources were acquired or retained. Respond only to a concrete owned correction from the original design consumer; independent integration retains whole-Delivery acceptance.

