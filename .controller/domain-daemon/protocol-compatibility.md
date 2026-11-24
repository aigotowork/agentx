---
schema: task
status: completed
agent: dani
description: Allow compatible client and daemon builds without mandatory restart.
---

# Protocol compatibility

Current native IPC uses epoch 4; see the [compatibility model](../../.docs/model/state-ownership.md#native-build-compatibility) and [one-shot reminder implementation](../time-reminders/implementation.task.md). The earlier epoch-3 [metadata verification](../agent-metadata/verification.md) and the epoch-2 rollout receipts below are historical and do not imply current-source adoption on existing services.

## Selected contract

The user requested replacing exact-build rejection with compatibility-based reuse. Business and inspection IPC compare a shared protocol epoch and retain canonical scope, authenticated peer, daemon instance and database generation checks. Package version and build cohort remain diagnostic fields. Epoch 2 establishes the maintained compatibility baseline; historical epoch 1 builds are not assumed compatible. Breaking native wire or operation semantics require an epoch bump. Compatible changes must preserve old callers and readers; no per-feature negotiation or strict-mode configuration is introduced in this bounded first slice.

## Ownership and cases

`src/daemon/wire.rs` owns the epoch, `transport.rs`/`server.rs` own ordinary handshake, and `observation.rs` owns inspection handshake. Cases: accept differing package/build on both client and server paths, execute a native call, reject wrong epoch/scope/peer and incomplete identity, retain instance/generation fencing. Update historical build-rejection fixtures to the new contract. Test disposable scopes only; do not replace or restart live user resources. Build a separately named candidate after focused checks.

## Evidence

Implementation is complete. `just test --bin agentx daemon:: -- --test-threads=1` passed 44 cases, with 17 environment-dependent cases explicitly ignored. The four new compatibility cases cover ordinary and inspection Hello/Ready acceptance across build/package differences, an actual native request, and rejection of invalid ordinary protocol/scope/PID/empty era identity. Existing cases retain generation, instance, deadline and lifecycle coverage. Scoped Rust formatting and `git diff --check` passed.

`just package .bin/agentx-compatible` produced AgentX 0.7.31 with protocol epoch 2. Build duration was 13m 10s. SHA-256: `f5246648e6712ac00b5b729d3522134a76800dc67d69e20bc0053312392df02e`; loaded diagnostic cohort: `native-build-v1:07058bfc-9746-4ddd-a471-3e3d913ef82e`.

The fixed debug test harness `target/debug/deps/agentx-7ac23d5cbc37f478` executed these selected ignored cases with `AGENTX_ACCEPTANCE_BINARY` pointing to that final release artifact; both passed:

- `daemon::acceptance_tests::process_protocol_mismatch_rejects_and_compatible_builds_connect` (4.22s).
- `daemon::acceptance_tests::process_native_cohort_copy_audit_and_running_path_replacement` (17.63s): the running copied daemon retained PID 1462193 and its loaded build identity after pathname replacement; it accepted a same-protocol Hello bearing the separately built harness cohort.

`python3 tests/tmux_headless.py .bin/agentx-compatible` also passed foreground/reattach, detached variants, config/conflicts, scoped sockets, retained provider PID, neighbor isolation and native override, including fixture cleanup. Only disposable test resources were started/stopped. `.bin/agentx`, `.bin/agentx-tmux`, installed programs, live daemon/workbench and user tmux sessions were preserved.

Native execution evidence is Linux-local. macOS/Windows runtime behavior is not independently established here. Capability negotiation and strict configuration are not implemented. Transition from the existing epoch-1 daemon requires one explicit operator restart; subsequent same-epoch rebuilds do not. This task supersedes the exact-cohort policy in the prior daemon delivery; its old acceptance remains historical evidence only.

## Encountered test maintenance

The first daemon-suite run passed all four new compatibility cases, but found two existing fixture/assertion defects: passive-empty-scope inherited a nonprivate temp-directory mode, and the artifact-read failure case expected `Unavailable` although native I/O maps to `DaemonError::Io`. Correct the fixture to mode 0700 on Unix and assert the existing typed I/O error; no production permissions or error mapping changes.


## Authorized project rollout

The user subsequently requested promoting the candidate to `agentx` and restarting the project daemon. The project PATH already resolved `agentx` to `/home/long/works/longwork/agentx/.bin/agentx`. Promoted the exact accepted artifact atomically without rebuilding; SHA-256 remains `f5246648e6712ac00b5b729d3522134a76800dc67d69e20bc0053312392df02e`. Prior executable retained at `.bin/backups/agentx-before-protocol2-b4e68d998c7ccc18`. The unrelated global `~/.local/bin/agentx` was not replaced.

Native `daemon restart --scope /home/long/works/longwork/agentx/.agentx` completed successfully. Old PID 1176754 exited; new PID 1497974 serves instance `6e932278-d303-46ac-863a-4697beab7c16` with epoch-2 build `native-build-v1:07058bfc-9746-4ddd-a471-3e3d913ef82e`. Subsequent native status confirmed running and a successful authenticated handshake.

The existing collaboration workbench at `0.0.0.0:18947` remained an old protocol client and returned `available:false` after the daemon switch. Restarted that affected client with its original cwd/environment, collaboration mode, host/port and log, translating the renamed `--name vivo` startup flag to `--iam vivo`. Old PID 177063 exited; new PID 1518707 runs the promoted executable. `/api/v1/connection` now reports `available:true` and generation `a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb`; a native read through `/api/v1/read` returned a successful `Read.List` result. Log remains `.agentx/.tmp/workbench.log`. Initial manual read used an untagged integer and correctly received HTTP 400; corrected to the existing lossless `{$integer:"1"}` browser wire contract for the successful read.

Four existing old-image PTY hosts (dani/mika/kera/neko; observed PIDs 885796/965435/1002753/1112123) were deliberately left running to avoid interrupting active agents. Their host IPC still uses epoch 1; reopening those sessions later is required to migrate the host process. User was informed of that limitation. Other daemon scopes and tmux sessions were not restarted.
