---
schema: task
status: completed
agent: Observation events, correlation and retention designer
---

# Observation events, correlation and retention designer

## Goal and professional authority

Define an observation stream that correlates client/server activity and references durable domain/host evidence while preserving distinct facts, cursors and retention lifetimes.

Read the [team contract](../AGENTS.md), [Delivery](../delivery.md), [proposal](../../../.docs/proposals/agentx-observability.md) and [actual bindings](../controller.md#runtime-bindings). Apply: Event/domain contract design and data/state modeling, with explicit ordering, replay, boundedness and authority semantics. Current source is evidence; planned contracts must be labeled as design targets.

## Inputs

`src/events/{mod,store,history_store,observer_store}.rs`, `src/db/events.rs`, `src/cli/commands/events_client.rs`, existing domain event/receipt offerings, runtime/configuration contract drafts and shared identity semantics.

## Required result

1. Source-ground current event/history/observer behavior and name the new observation responsibility. Separate transport/runtime telemetry, domain events and owner-reported host effects; do not duplicate business truth or conflate telemetry with a domain audit log.

2. Define concrete event envelope and typed payload families, stream/epoch/cursor identities, actor/request-attempt/domain-object references, source and observed/occurred time meaning, severity/outcome/freshness and dropped/gap metadata. Do not turn a lost reply into failure/completion or request identity into replay authority.

3. Specify query/subscription or poll/wake operations and results/errors, bounded pagination/filter consistency, snapshot/continuation semantics, resumption, gap/reset markers and cancellation/cleanup. Preserve short operation IPC semantics or justify a minimal changed transport boundary from actual consumer needs.

4. Define memory/durable retention and update/serialization ownership, collection pressure/drop policy, cardinality/size budgets, redaction, late or duplicate reports, local versus cross-source ordering, reset/generation and daemon-restart behavior. Keep current archive cursors and source IDs independent where their semantics differ.

5. Coordinate producers/consumers with runtime and configuration before final release. Explain the evidence chain from request acceptance through domain commit to actual host confirmation, with partial visibility rather than a fabricated total-order global trace.

6. Produce typed examples and meaningful scenario fixtures for replay, gaps, source replacement, restart/reset, slow consumers, uncertain outcomes and inaccessible client/host evidence. Name planned source/contract/adapter homes and implementation obligations.

## Write and resource boundary

Own `.docs/observability/events.md` and this original Task. Coordinate shared-file transfers explicitly. Root navigation, roadmap and proposal/Delivery state remain controller-owned. Read-only current source inspection is authorized. No production source/dependency/package changes, no build/test/runtime probes, no nested agents, and no live/default scope access.

## Handoff and completion

Runtime/configuration consume correlation and producer boundaries; architecture integrates shared references; integration consumes stream/cursor/error contracts and investigation cases.

Release usable contracts early when they unblock a real consumer. Normal progress stays here; message only actionable contracts, concrete questions/findings or completion through AgentX. The final handoff must identify authoritative artifacts, resolved versus deferred decisions, scenario evidence, downstream obligations and released write/resources. Design completion never claims executable support.

## Working record

2026-11-07: Entered the event/domain contract and data/state modeling methods. Scope is the new observation capability, preserving the completed daemon baseline and native domain/host authority. Work proceeds through current-source reconstruction, a bounded stream/correlation/retention model, early producer/consumer contract release, then focused scenario handoff. Planned declarations belong in the owned design until separately authorized implementation promotes them to native source. No production changes, compilers, runtime probes or nested agents are in scope.

This original Task remains the execution authority; Paseo suggestion `UNS-6` links the work to the session board.

Source reconstruction completed for native events/service/store/history/observer and CLI consumption, DB append/postcommit/reset, caller context, wire/server/frame and message acceptance/inbox receipts. Consequential current facts are linked in [the design baseline](../../../.docs/observability/events.md#source-grounded-baseline): history cursor ordering spans independent frozen sources, postcommit notifications are advisory, and durable native acceptance/claim/receipt authority cannot be inferred from reply or telemetry presence.

Selected a volatile bounded stream, no telemetry table/outbox, no server subscription/wait, and fixed-frontier short polling. [The initial read contract](../../../.docs/observability/events.md#stream-identity-and-read-contract) was released to integration with exact scanned-cursor semantics, distinct no-match/gap/capture-drop meanings, and no automatic old-era rebinding. Runtime accepted the nonblocking sink and bounds (8,192 records, 16 MiB, 16 KiB/record, 15 min) and will update its registry before recording. Both owners agreed that existing host receipt/claim evidence remains canonical; no generic host-effect report is introduced.

Architecture and integration identified an early naming/home mismatch. Corrected it at the owner: import shared `ObservationEpoch`/`ObservationTime`, keep the collector nonce in separate `ObservationStreamId`, and use planned `src/observability/events.rs` plus the passive inspection lane. No product source changed.

Published concrete envelope/payload imports, domain evidence references and actual native lookup limits, filters/errors, loss samples, lifetime/rotation transitions and bounded preparation/serialization rules. Source-backed correction from architecture: native `OutputToken.id` plus participant/through would reconstruct confirmation input (`messaging/service_store.rs::ConfirmOutput`/`confirm_output_and_status`); the final projection uses fresh `OutputObservationId` with no reverse mapping. Full delivery receipt/frontier/turn is also omitted in favor of claim + boundary + confirmation disposition. No telemetry value is offered as a new mutation capability.

Both runtime/configuration briefly errored due to provider high demand; the controller handled bounded resume. Their actual released documents are now consumed, not inferred from chat: RuntimeEvent/RuntimeCorrelation, terminal ClientAttemptReport and source `ended_at`; exact ConfigObservationEvent/ConfigSnapshotRef and independent snapshot lifetime. Runtime accepted `ClientReportObservation { report, submitted_by: AcceptedPeer }`, constructed only at current authenticated intake, preserving historical client claims separately from current submitter evidence. RequestObservationId filtering is explicitly qualified by the cursor's daemon; source-era reports never match a server request by naked sequence/UUID.

The [owned fixture set](../../../.docs/observability/events.md#typed-scenario-fixtures-and-read-traces) covers fixed-frontier append/replay, scanned continuation/no-match/gap, restart/reset/collector replacement, bounded loss/slow readers, lost reply and inaccessible host/client/config evidence, archive source replacement, duplicate claims and cancellation. These are manual design traces, not test execution. One scoped `git diff --check` invocation returned no diagnostics, but these new artifacts were untracked and that result does not validate their new bytes or contracts. Independent integration owns final document/contract checks and complete scenario consumption.

## Completion and handoff

Released the complete [Events design](../../../.docs/observability/events.md) to architecture/integration and producer owners. Consumable boundaries are collection/sink, stream/read, envelope/correlation, domain/host evidence, filters/errors/losses, retention/state and typed traces. All event-owned questions in the assignment are resolved at design scope; no unresolved Runtime/configuration import inference remains. Chosen alternatives and their reasons are retained in the design: volatile retention instead of a durable feed/outbox, short passive polling instead of waiting IPC/subscription state, immutable append order instead of source-wall-time ordering, bounded loss samples instead of a global complete trace, and native owner readback instead of a fabricated generic receipt lookup.

No executable capability is claimed. Downstream owners must promote declarations, instrument actual transitions/commit boundaries, implement capped serialization/rotation/pressure handling, and establish the [mapped evidence obligations](../../../.docs/observability/events.md#planned-implementation-obligations-and-evidence-limits). Independent integration retains whole-design acceptance. Both owned files are released; no locks, subprocesses, terminals, service instances, package jobs, live scopes, native source writes or sibling-file writes are held.

