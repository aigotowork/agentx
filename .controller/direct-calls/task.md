---
schema: task
status: active
description: Replace compatibility handshakes with directly fenced calls.
---

# Integration and native lane

The [approved plan](../../.codex/attachments/945e02cd-407e-42db-9b37-5c5188ad3d7b/approved-plan.md) sets scope. Caller identity, daemon process instance, and durable generation are separate. Native transport authenticates kernel peer; wire handles bounded decode and operation compatibility; server owns scope, authority, fences and dispatch; service client preserves caller context and IDs; lifecycle alone may speak published legacy 3/4 to stop a verified instance. Inspection is a separate passive lane. No automatic retries after a write may have executed. No installed or live changes during implementation.

Local SDK/Space and cloud own their boundary details in the linked tasks. Integration checks real native migration, finite TS/Web cases and generated declarations, and records platform gaps.


## Trace finding

The first full-CLI socket trace found an unconditional Development Root read before submit, even after native handshakes were gone. Re-exec is a local executable selection. The daemon now projects the durable dev-root setting to its private lifecycle directory during startup, explicit changes and data reset; the CLI reads that file without IPC. The scope database remains the durable setting and startup backfills it on the one-time upgrade. This avoids a Maintenance.DevRoot business request on every CLI invocation. The [retained syscall trace](../../.artifacts/direct-calls/hot-send.strace) then verified one connect, one Call and one Result in an actual quiet CLI send, with one persisted event.


## Integration evidence

The first [combined candidate](../../.artifacts/direct-calls/agentx-candidate) has SHA-256 `6ec5f3e45e3c794231fd22c712f4bcaf9236fcd68b289b9862cf854516804c4a`. Its Management and both Cloud OpenAPI exports compare byte-for-byte with the generated input specs. The embedded Cloud Space [native API scenario](../../.artifacts/direct-calls/cloud-space-native/result.json) and [native Management HTTP conformance](../../.artifacts/direct-calls/native-http/native-result.json) passed. Management rejected a wrong Scope with the correct generation before admission; the original request ID had no receipt.

- Linux x86-64 migration used the verified published v0.7.31 and v0.7.33 archives. Both older daemon children drained with a successful exit, a neighboring owner's status and process stayed unchanged, and the new process preserved the original generation, message event and request receipt. v0.7.31 requires a test-local process binding to make the named manual registration; the setup shim used for the earlier harness only injected that binding on `start` and exec'd the original release executable. The [portable fixture](../../src/daemon/upgrade_acceptance_tests.rs) now supplies it directly.
- The daemon/Inspection suite passed 49 cases (22 opt-in process cases skipped by default); [35 follow-up native cases](../../.artifacts/direct-calls/native-focused.log) passed over Management, reset, events, dev-root and exact host poll evidence. See also [local browser/SDK](local.task.md) and [cloud HTTPS, browser and TUI](cloud.task.md).
- The legacy startup fixture passed cold CLI and recovery cases SU1–SU5, then caught a cold Workbench regression: selecting a lazy Client no longer started the daemon as a side effect. Workbench now explicitly acquires readiness at its local startup while its HTTP resource calls remain passive. The [embedded candidate](../../.artifacts/direct-calls/agentx-final) (SHA-256 `f6119096c87fb900d3e1609d1657e2a3b0706b32a8ca3915f80487bdeaed28cf`) passed both [Workbench startup cases](../../.artifacts/direct-calls/startup/final-workbench/resources.json) and a newly executed [two-host embedded HTTPS browser scenario](../../.artifacts/direct-calls/cloud-https-final/result.json). It includes the latest Cloud Space draft and missing-history presentation and emits all three identical OpenAPI contracts.
- A real follower process exposed a retained-client restart regression: the lazy binding could reacquire after an already served daemon exited. The Client now consumes auto-start permission only at first acquisition, shared by its clones. A first lost write reply also consumes that permission. The transport continues to allow a fresh operator invocation to cold-start under the existing lock. The retained-client test asserts that daemon loss does not create a startup lock or recreate managed storage.
- The first build after that fix encountered the shared-ignored SDK output race already recorded in the [cloud handoff](cloud.task.md). Serializing the integration rebuild avoids using partially generated browser imports; no cached or incomplete bundle should be shipped.
- This host is Linux x86-64. Apple Silicon and Windows runtime checks remain open. The public v0.7.31 release lists Linux assets only, so the matching platform binaries and runners must be supplied for those two executions. The source uses the same authenticated native Shutdown semantics on all platforms.

## Verified Linux artifact

[agentx-verified](../../.artifacts/direct-calls/agentx-verified) includes the retained-client startup fix and latest embedded apps. SHA-256: `9632eede348241af8a8b6af65fd7e49cd9eca3e56d57405761a4f5bcdff562cf`. Its three OpenAPI exports match the checked-in generator inputs byte-for-byte. The [final socket trace](../../.artifacts/direct-calls/hot-send.strace) shows one Call/Result. The [full startup acceptance](../../.artifacts/direct-calls/startup/verified/resources.json) passed SU1–SU7, including cold Workbench, retained singleton inode, wrong-peer refusal and clean disposal. The [published migration test](../../.artifacts/direct-calls/published-upgrade.log) now runs both original binaries directly with no wrapper, and the [final native suite](../../.artifacts/direct-calls/native-verified-tests.log) passed 63 affected checks. The [real restart/reset process case](../../.artifacts/direct-calls/native-restart-verified.log) also passed: the follower reported daemon loss without restarting it, independent restart retained the generation, and reset rejected the old client.

On a new platform runner, set `AGENTX_ACCEPTANCE_BINARY`, `AGENTX_UPGRADE_V0731`, and `AGENTX_UPGRADE_V0733` to absolute paths to the candidate and the two verified originals, then run `cargo test --locked --bin agentx published_daemons_drain_without_changing_the_neighbor_or_old_receipt -- --ignored --nocapture`. The fixture creates and cleans both selected and neighboring scopes.

## Rollout order

Publish/deploy the generation-validating Cloud Namespace service before switching an SDK or embedded UI that relies on it. For a selected local scope, the operator runs `agentx daemon restart --scope PATH` once with the new executable, then reopens Agents still running a legacy host. Running instances and installed binaries were left in place throughout implementation and verification.
