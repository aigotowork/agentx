---
schema: task
status: completed
description: Integrate lightweight hook telemetry and make pressure diagnosable without production changes.
links: [../../.docs/observability/telemetry-coverage.md, ./implementation.md]
---

# Telemetry optimization

The human authorized documentation and implementation. This follow-up is owned inline by bele because native correlation, CLI lookup and file evidence share one small contract and active working-tree changes. Preserve existing source changes, installed executables and live daemons.

Implement the first end-to-end slice in the [coverage design](../../.docs/observability/telemetry-coverage.md): Codex/Claude local timing and sampling, shared invocation correlation for native requests, a passive trace lookup and native pressure accounting. Verify the actual binary with disposable scopes and a held daemon dispatcher or existing congestion tests; measure capture overhead separately from provider latency wherever the fixture permits. Plan remaining producer enrollment in the coverage design. Earlier free-form hook timing records mix builds and did not log admission vs application time; use them only as evidence of the diagnostic gap, not as an A/B benchmark.

The first slice changes no database schema or business wire metadata. Existing origin UUIDs and revisions join the evidence, and the new native read remains off the public browser request surface. A confirmed `Busy` during daemon Hello now returns without cold acquisition; an I/O error without a received result keeps the existing recovery rules. In the ordinary listener, reap idle-completed workers before classifying a full pool.

Remaining rollout work follows the producer boundaries in the [coverage design](../../.docs/observability/telemetry-coverage.md#agentx-telemetry-coverage-and-optimization). Keep the first slice usable while extending it. In particular, other Providers may use different persistent processes or callback contracts, and short-lived origins cannot produce a full population histogram without an explicit aggregation design. No deployment or restart is part of this task.

## Implemented and verified

The [native timing collector](../../src/observability/trace.rs) replaced per-dispatch timing log writes for ordinary Codex/Claude hooks. The [CLI trace read](../../src/cli/commands/observe/trace.rs) joins local summaries with retained native requests, and `observe inspect` now displays the existing latency histograms and pressure counters. The [native dispatcher](../../src/daemon/server.rs) reports queue wait and application time separately and counts pre-decode connection pressure. A real saturation attempt first exposed a Hello write/close race: even with the explicit `Busy` branch fixed, a buffered rejection could be mistaken for I/O failure. The [transport](../../src/daemon/transport.rs) now accepts that bounded same-connection `Busy` frame after an early disconnect, preserving the original error otherwise.

Relevant Rust checks passed: 78 observation tests and 41 selected daemon/CLI/log tests, with three existing opt-in product tests ignored. After the final Hello change the isolated Linux executable at `target/telemetry-check/debug/agentx` was rebuilt and the [real process fixture](../../tests/observability_hook_trace.py) passed. The final run joined four native requests to one Codex Hook, retained Codex/Claude invalid-input diagnoses with no daemon or payload disclosure, measured 15.88 ms wall time for a Hook against a deliberately full business pool with no startup lock/spawn, and recorded zero additional log bytes for four quick hooks at 0% normal sampling. A deliberately delayed hook was retained at 775.78 ms even at 0%. These are finite Linux fixture results, not production latency or an instrumentation CPU benchmark. Source whitespace and documentation links were checked. Installed binaries and live daemons were not changed.
