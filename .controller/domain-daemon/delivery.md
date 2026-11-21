---
schema: delivery
description: Establish and deliver AgentX domain ownership with a single-binary client/daemon for each data scope.
status: completed
stages:
  model:
    description: Reconstruct source behavior, domain state ownership, client scenarios and complete database-access boundary.
    status: completed
    tasks: [./tasks/architecture.md]
  contract:
    description: Establish the first complete Rust-native application operations and local transport/lifecycle contract.
    status: completed
    depends_on: [{stage: model, condition: domain owners and highest-risk unknowns are source-grounded}]
  daemon:
    description: Implement one-binary daemon discovery, singleton startup, readiness, IPC and exclusive persistence composition.
    status: completed
    tasks: [./tasks/daemon.md]
    depends_on: [{stage: contract, condition: transport and process-lifecycle contracts are accepted}]
  domains:
    description: Migrate participant/session identity, messaging/delivery and activation/provider paths behind the daemon.
    status: completed
    tasks: [./tasks/consistency.md, ./tasks/runtime.md, ./tasks/clients.md, ./tasks/relay.md]
    depends_on: [{stage: contract, condition: required domain and host native contracts are released to owners}]
  cutover:
    description: Move all remaining callers, eliminate production direct database access, and verify restart/upgrade/performance behavior.
    status: completed
    tasks: [./tasks/integration.md]
    depends_on: [{stage: domains, condition: all supported domain and provider entry points have migrated}]
---

# Delivery

## Accepted local outcome

The controller accepts this Delivery's design, implementation and local Linux integration as completed. The [final integration receipt](tasks/integration.md#final-packaged-artifact-and-local-acceptance) and [current acceptance](../../.docs/architecture/acceptance.md#current-local-outcome) own the exact source, commands, artifacts, coverage and cleanup. Final `candidate-s10-lint-source` contains 417 fixed inputs, remained unchanged through packaging and matches current workspace build inputs. The local [`.bin/agentx`](../../.bin/agentx) is `agentx 0.7.31`; required formatting, all-target locked Clippy with warnings denied, strict ownership enforcement and `just package` passed. Strict ownership found zero violations across 335 Rust sources and 1,246 lexical findings. The actual packaged Events case returned all 2,014 current/archive events and confirmed warning/order/history cleanup behavior with matching-daemon-only readable managed descriptors.

Final same-runner DD12 completed with zero operation errors and all fixture processes/roots removed. Fresh-scope list is 30.009 ms, sequential list p50/p95 is 7.145/8.599 ms and manual start is 32.476/40.781 ms. Cold and start remain +13.340 ms and +7.122/+6.343 ms above the preserved baseline; these disclosed costs are accepted for the local implementation, without a parity or speedup claim. Earlier selected contract/host/Provider/process passes retain their own immutable-input scopes and were not repeated as final-artifact executions. Fourteen item-local enum-size expectations retain their documented native layout costs.

| Acceptance boundary | Final disposition |
| --- | --- |
| Local implementation/build/isolated acceptance | Completed on Linux x86_64 with Rust 1.99 and controlled local fixtures, including the final package, exact source guard and measured workload. All source/build/test/process resources are released; no further edit, rerun or tuning is selected. |
| Native platforms and toolchain matrix | Native macOS/Windows, MSRV 1.89 and pinned Node 22/npm 10 execution remain unverified. Source implementation and local fixtures do not certify them. |
| External integrations | Vendor SDK/model sessions and live remote peers remain unverified; controlled Provider and embedded-consumer cases retain their explicit limits. |
| Installed/live deployment | Installation, PATH replacement, publishing and live service cutover are separate outcomes and were not performed. Old binaries that ignore the new scope lease still require the [supported legacy retirement procedure](../../.docs/model/state-ownership.md#legacy-process-cutover); mixed reintroduction is unsupported. |

## Target outcome

AgentX uses explicit domain-owned state and native Rust contracts. One `agentx` executable starts both the daemon and client entry points. For each canonical AgentX data scope, the daemon is the sole production database owner; all other application entry points call it through typed use cases and transport adapters. Provider and Paseo identities and protocols remain in their existing ownership layers.

The target and migration constraints are defined by the [proposal](../../.docs/proposals/domain-driven-agentx-daemon.md) and navigated through the [current/target model](../../.docs/model/README.md). Delivery completion requires all supported DB-backed entry points to use the service boundary, not merely a runnable daemon command.

## Acceptance boundary

- Exactly one active daemon can own a canonical data scope under concurrent client starts; clients discover readiness and negotiate compatible protocol/build capabilities.
- Only daemon composition opens/migrates/resets the production SQLite database. No client or provider host silently falls back to direct SQLite after IPC failure.
- Identity, provider-session environment and execution capabilities are explicit per request/session and cannot leak from daemon-global environment or another session.
- AgentX native contracts retain accepted/committed/observed effect distinctions, stable retry/reconciliation semantics, durable cursors and current identity/attachment/claim fences.
- CLI, TUI, hooks, plugins, relay, PTY/runtime hosts, Codex and other supported Providers keep their supported behavior through domain-owned adapters.
- Daemon failure/restart, reset/database generation replacement, stale callers, old client compatibility and event backpressure have defined outcomes.
- Performance is described from a fixed baseline and representative workload; exclusive DB ownership alone is not evidence of higher throughput or lower latency.
- A clean build and scoped contract/process/provider acceptance demonstrate the declared platform matrix. Installation, release and changes to live user services remain separate work.

## Stage notes

The `domains` migration stage is accepted for its source-owned implementation and independently selected native/host evidence, including the [s10 Events closure](tasks/integration.md#coherent-s10-events-and-final-check-input):19new archive/history/observer/actual TestDaemon host cases passed after bounded compiler corrections. All production entry-point and final warning-cleanup source claims are released. Earlier domain/Provider/process receipts retain their own fixed-input scopes; the final local `cutover` receipt now supplies the required lint, package, guard and artifact performance result. Platform limits remain as declared above.

The `cutover` stage is accepted for local source and isolated-process acceptance on the final packaged artifact. Central formatting, bounded diagnostic overlays, exact read-only archive constructor policy, clean all-target Clippy, strict guard, actual packaged CLI and final-artifact DD12 are complete. Earlier snapshots and failures remain immutable; the [final receipt](tasks/integration.md#final-packaged-artifact-and-local-acceptance) distinguishes prior selected passes from checks actually run on final bytes. Live service cutover was not performed.

The [s9 affected result](tasks/integration.md#s9-actual-product-closure-and-derived-production-correction) is accepted within its documented local process/native-fixture scope:19native/host cases and2actual Codex/Copilot hook/extension cases passed, with the derived strict ownership guard passing and exact fixture process/root cleanup confirmed. That executable contains only the source-released erroneous cfg gate removal beyond frozen s9; the fixture correction is separately retained. These results retain that scope rather than becoming final-artifact executions.

The bounded `daemon` stage is completed and accepted for its documented local Linux/native-fixture scope: cohort compatibility/generator behavior, independent listener admission, sole application dispatch, eight lifecycle/scope and three affected process cases, and measured correction of the material warm-read regression. The [daemon Task](tasks/daemon.md) retains exact evidence, cold/start residuals and resource handoff. Final local combined-source ownership/performance/package acceptance is recorded above; native platform execution remains unverified.

`model` has delivered the source-grounded vocabulary/state ownership, fixed-baseline DB inventory and model/contract navigation. Architecture remains available for concrete integration corrections. `contract` turns those scenarios into Rust declarations and the actual transport binding. Released native boundaries permit `daemon` and `domains` to implement independent cohesive slices concurrently; that readiness is distinct from final acceptance. `daemon` must prove lifecycle and one cooperating database-backed capability in an isolated scope. `domains` retain supported Provider/client behavior. `cutover` removes legacy database composition and proves the combined process, domain, platform and workload outcome. Any incompatibility or unmet contract returns to the owning stage and updates the proposal/model before a cross-layer workaround is added.

## Next product milestones

These outcome checkpoints are complete within the accepted local scope. Each earlier functional slice retains its documented Linux controlled-fixture/process input; the final local package and performance result are separately accepted above. The [controller](controller.md#execution-focus-after-the-user-requested-efficiency-assessment) owns assignment closure; original Tasks retain implementation detail and the acceptance ledger retains execution evidence.

| Milestone | Required observable outcome | Ownership and remaining gate |
| --- | --- | --- |
| CLI and daemon | Fixed product executable in a disposable scope: cold client acquires the daemon; real `list`, participant start, message send and event observation use native IPC; concurrent clients share that daemon; DB-handle observation covers every participating process; daemon loss produces the defined failure without client DB fallback | Functionally accepted for fixed s5 release: seven prior process passes plus the corrected origin case, rerun against the same product with only fixture changes. Readable-handle/PID limits remain explicit. Ordinary CLI migration and performance are not accepted by this slice. |
| Child runtime and activation | A real host launched through the product path uses explicit participant/session environment, exchanges a message with a child and delivers an idle wake through a controlled Provider fixture; exact receipts and restart/reconnect fences retain their declared semantics | Functionally accepted on the fixed s7 product: actual CLI/headless runner/PTY Qoder fixture, inherited identity/generation/cwd/session, bidirectional sends, listening-child wake, actual hook flush/exact cursor and scoped FD/cleanup evidence. This is a controlled Provider fixture, not paid/native model or performance evidence. Existing s5 reset/restart evidence retains its own snapshot scope. |
| Complete cutover | All supported remaining CLI/TUI/hook/plugin/PTY/stdio/relay paths migrate; final production access guard, restart/reset, supported legacy retirement, platform limits, cold/warm comparison and local package are evidenced against final candidate bytes | Accepted for the final local package and disclosed Linux fixture/measurement scope. Exact final source/build/guard/package/Events/DD12 attribution is in the final integration receipt; earlier case families retain their own inputs. Native platforms, vendor/live-peer execution and live service cutover remain unverified or separate outcomes as declared above. |

All assignments and resources are released. No additional implementation, review, check or tuning cycle is selected; a future concrete defect or broader platform/deployment request needs its own bounded assignment while preserving this evidence.
