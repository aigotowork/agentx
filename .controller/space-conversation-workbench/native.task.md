---
schema: task
status: completed
title: Native message history and SDK
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"muzeko","incarnation_bits":4745316636965283225}}]
---

# Native history and public SDK owner

Own src/ native db/Management/daemon plus packages/agentx-client and generated OpenAPI only as needed, including focused verification. Read the [approved plan](AGENTS.md) and [shared contract](delivery.md). Reuse existing conversation index, exact bounds and byte paging. Coverage decisions and watermark must stay consistent with concurrent writes; bounded raw fallback must not report an incomplete range as empty. Add old daemon capability negotiation without breaking native protocol compatibility and retain /messages behavior. Record exact tests and make a debug native candidate available to the Controller; coordinate heavyweight builds and keep resource use low. Earlier user stopped 750k load jobs: use modest sparse deterministic test data. No existing listener/package changes. Communicate integration blockers and final evidence.

## Native design

The Management read lane pins one WAL snapshot for generation, exact instance bounds, earliest scanned range, membership coverage, latest-page event watermark, and attachment projection. The existing journal and conversation index stay authoritative. The events adapter reads a descending page within the captured upper bound, then returns it in ascending ID order. The existing event pager enforces count and byte limits; the oldest returned ID is the exclusive next bound. The raw fallback inspects at most 4,096 journal rows per request. If that cannot establish a complete page or prove older history exists, it returns retryable Unavailable rather than reporting a false end of history. Older page reads never return a live watermark.

Management owns the new native request, HTTP binding and generated schema. The daemon's optional readiness bit gates the actual IPC exchange on the call connection, leaving protocol epoch 3 and forward messages intact. The SDK binds the history cursor and live checkpoint to the selected generation and exact current/stopped reference. The scope descriptor owns feature selection.

## Build coordination

The first debug build exposed a bootstrap dependency: embedded Space already imports the new SDK shape, while SDK generation requires the native export. A temporary schema based on the Rust declarations bootstrapped the SDK and unblocked the shared Space typecheck. The native candidate's emitted Management document will replace it before final delivery. The single Cargo `-j2` candidate build encountered sustained host disk I/O pressure during Rust code generation. No duplicate Cargo build or scale fixture was started.

The build completed with `cargo build --offline -j2 --bin agentx`. The [frozen Linux debug candidate](../../.artifacts/space-conversation-workbench/native/agentx-history-debug) has SHA-256 `bbfa874d8651610f300f2466cb0daa8591184fd80a84a6070e935fa885690256`. The [actual export](../../.artifacts/space-conversation-workbench/native/management.json) replaced the bootstrap source in [Management OpenAPI](../../.docs/api-panorama/openapi/management.json) and regenerated the SDK. All 28 existing path definitions compare equal; the only schema changes are Descriptor's optional history marker, LaunchRelations's optional fork source, and the new history page. Generated Workbench and Observation files compare byte for byte equal. The candidate is for disposable native integration; its embedded assets are from the build's source snapshot.

The actual export passed the [SDK build and all 25 tests](../../.artifacts/space-conversation-workbench/native/sdk-test.log). The new cases exercise latest and older queries, cursor kinds, exact reference and generation isolation, an unchanged legacy descriptor/forward read, and independent Fork source. The [SQL query plans](../../.artifacts/space-conversation-workbench/native/sql-plan.txt) independently confirm a descending participant-index seek and a bounded journal rowid seek, without a temporary sort.


The [nine focused Rust cases](../../.artifacts/space-conversation-workbench/native/rust-test.log) passed: `CARGO_INCREMENTAL=0 cargo test --offline -j2 --bin agentx -- message_history exact_conversation_index_preserves_pages launch_relations_real_resume native_completeness_old_daemon --test-threads=1`. Coverage includes the pre-read watermark under WAL append/backfill, follow-on forward delivery, exact stopped/current and generation isolation, default page size, byte limits, capped fallback, old-daemon negotiation, generated cursors, and distinct Fork creator/source. The sparse fixture uses 4,096 unrelated rows to cross the fallback boundary. This is a focused suite, not a scale run.

The Controller's [isolated native + SDK + Chromium report](../../.artifacts/space-conversation-workbench/native-composed-r1/browser-report.json) passed the 20/20/7 split and the unchanged forward path, latest-to-older browser paging, and exact Fork provenance. Installed executables, packages and existing listeners were not changed by this Task.
