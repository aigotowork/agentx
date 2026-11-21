---
schema: document
status: completed
owner: mara
links: [repair.task.md, diagnosis.md, usage.md, ../../.artifacts/message-delivery/candidate-r2-receipt.json]
---

# Message delivery correction result

The selected bounded repair is implemented and verified on a separate immutable candidate. Ordinary `agentx send` reaches the exact waiting controlled Codex and Qoder processes, queued messages survive compatible daemon restart, and explicit native kill/resume delivers pre-retirement backlog into the retained Provider session. No manual `term inject`, duplicate submission, live/default adoption or subagents were used. Sole Cargo/check/package lane is released.

## Supported cause and correction

Original protocol2 host images repeatedly fail authenticated Hello against protocol3 daemon PID3400255; all four original names have correlated snapshot failures. A new executable on disk cannot update those loaded hosts. Original bounded logs/events remain in `.artifacts/message-delivery/original-host-failures.json` and `original-events.jsonl`. This establishes that delivery stops at the host snapshot/compatibility boundary after that transition; the earlier incident timeline remains incompletely established.

The transport uses one-shot local IPC, durable SQLite backlog and best-effort connect-close TCP hints plus 30-second idle polling. There is no persistent daemon subscription whose broken socket explains these observations. A socket redesign does not repair incompatible loaded code. No same-epoch durable-backlog loss was reproduced in the executed cases.

Production correction makes acceptance versus consumption truthful and actionable: ordinary send includes an event-correlated report; `send --delivery-status EVENT --json` reads that report without submitting, draining or changing caller activity. Exact frozen recipients prevent namesake-successor credit. Registered authenticated process polling records independent protocol/time/liveness evidence; legacy progress remains unknown. Acceptance still succeeds if subsequent observation fails. Mailbox confirmation never claims model execution. Native declarations are in `src/messaging/delivery_report.rs`, `src/messaging/service.rs` and `src/runtime/hosts.rs`; process progress is persisted by `process_host_store.rs` only for the matching authenticated peer/incarnation. CLI integration/help and module entries are updated.

Supported recovery is concrete and verified, not only a warning: retain the accepted event, inspect exact scope/participant/PID/Provider session, coordinate native kill/resume with a compatible image, then verify the retained session consumed the existing body. Exact recovery commands and gates are in [usage](usage.md). Old event reports remain retired rather than crediting a new incarnation. Original live hosts are preserved; nezi owns separately authorized coordinated adoption. This candidate does not hot-repair them. Fano's direct-socket architecture remains a separate design-only follow-up.

## Artifact identity

- Candidate: `.bin/agentx-message-delivery-r2`, agentx0.7.31.
- SHA256: `38aaeea800fa8023edb426fd8f25ccd9493875a824d29e0ceb94f47b818c97b9`.
- Native cohort: `native-build-v1:5b061157-ef07-4cf2-8814-a38ddc87c988`.
- Build: `CARGO_PROFILE_RELEASE_OPT_LEVEL=0 CARGO_PROFILE_RELEASE_LTO=false CARGO_PROFILE_RELEASE_CODEGEN_UNITS=256 just package .bin/agentx-message-delivery-r2`. Functional build; no optimized performance claim.
- Package log, all packaged input hashes, final harness hashes and test-only drift: `.artifacts/message-delivery/package-r2.log`, `candidate-inputs-r2.json`, `candidate-r2-receipt.json`. All production inputs match packaging; only `src/daemon/acceptance_tests.rs` and `tests/fixtures/daemon_qoder.py` changed afterward. R1 and accepted labels candidates are retained untouched.

## Executed verification

Final harness compilation: `cargo test --locked --bin agentx --no-run` passed (`test-build-final3.log`). Product is fixed by `AGENTX_ACCEPTANCE_BINARY="$PWD/.bin/agentx-message-delivery-r2"`; harness stamp is not substituted for product identity.

Each controlled test ran through `target/debug/deps/agentx-aa1d376cc9afeff5 TEST --exact --ignored --nocapture --test-threads=1`:

| Test | Result / retained log |
| --- | --- |
| `daemon::acceptance_tests::process_codex_message_delivery_idle_queue_and_reconnect` | PASS, 16.61s; `codex-final.log` |
| `daemon::acceptance_tests::process_qoder_real_launch_child_message_and_idle_wake` | PASS, 14.24s; `qoder-final3.log` |

Both tests assert ordinary send after completed turn, pending active backlog, compatible restart retaining process and DB generation, actual queued hook output, one activation per batch, caller lookup with no cursor/status/context changes, and supported native recovery preserving session and delivering the old body. Structured exact event/incarnation/PID/output evidence is retained in `codex-evidence.json` and `qoder-evidence.json` under `.artifacts/message-delivery/`.

Focused native harness filters also passed: `messaging::service_store::acceptance_tests` (7), `runtime::hosts_store::acceptance_tests::message_delivery_poll_progress_requires_registered_peer_and_preserves_legacy_unknown` (1), and `daemon::compatibility_tests` (2). Logs: `acceptance-units-final.log`, `host-progress-final.log`, `compatibility-final.log`. They cover replay/frozen recipients, rollback/receipt boundaries, authenticated-only progress/legacy unknown and scope/peer/generation/epoch fences in both Hello directions. Later test-only corrections do not alter these production guarantees; passes were not repeated. Candidate help includes delivery lookup. Narrow `git diff --check` passed.

Earlier failures are retained and classified in [diagnosis](diagnosis.md). Qoder Stop returns `decision:block` with queued context to continue a turn; it need not submit a terminal wake. The fixture now records that at its shared hook boundary, including the first Stop after resume. The correction preserves actual output assertions and does not alter production Provider semantics.

## Cleanup and limits

Fixture teardown fences exact disposable scope, executable/script and process incarnation before cleanup and asserts no owned launch processes/daemons remain. Final audit `.artifacts/message-delivery/cleanup-final.json` confirms recorded Codex PIDs971337/972897/971154/972383 and Qoder PIDs1081870/1083693/1081688/1083388 absent. Only each restarted fixture daemon held readable managed database descriptors before teardown; inaccessible processes remain explicitly recorded in evidence.

Execution used actual native AgentX daemon/CLI/PTY and generated hooks with controlled Python Provider substitutes on Linux. It does not establish paid/native model execution, all-platform behavior or universal liveness. No isolated missed-hint fault was injected; polling fallback is source-established while queue/reconnect/recovery were executed. Existing unrelated labels warnings remain. Original incident processes, live services, default binaries and live managed storage were not changed.
