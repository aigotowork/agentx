---
status: source-fixed-and-target-recovered
owner: messaging/PTY host
scope: humazo recursive screen-read deadlock and bounded recovery
related: [../../src/messaging/AGENTS.md, ../../.artifacts/humazo-deadlock/reproduction.json]
---

# humazo PTY host deadlock

## Cause established on the live process

On 2026-11-15, humazo's Codex PID 2308386 remained alive while PTY host PID 2308380 stopped servicing delivery, heartbeat and injection RPCs. The running image was `.bin/agentx-space-responsive-r1`, SHA256 `5e6843eedb521dfa5bfbb2cde983bc7ed8e78cf61ee2a9f5098a5600a36064e5`. The causal host.rs, mod.rs and shared.rs inputs matched that image's recorded source manifest before editing.

The gate-blocked diagnostic in `src/messaging/delivery/host.rs` held a screen read guard and called `DeliveryState::is_user_active()`, which acquired the same read lock again. If the PTY loop queued its screen write between those operations, Linux's writer-preferring RwLock blocked the second read behind that writer. The writer waited for the first read guard, owned by the blocked delivery thread. Both threads therefore waited forever. The occurrence depends on scheduling; a blocked gate alone does not guarantee deadlock.

Live debugger evidence: delivery thread 2308453 was in the read slow path through is_user_active; the PTY main thread was in the write slow path through update_delivery_state. The lock at `0x593c17507d20` held `0xc0000001` (one reader, readers and writers waiting). The caller's saved read guard referenced that same lock. Disassembly panic-location metadata resolved to `src/messaging/delivery/mod.rs:747:41`, the original recursive read. [Retained debugger capture](../../.artifacts/humazo-deadlock/live-gdb.txt) contains stacks, lock words and source filename; symbol names were resolved separately by disassembly because the binary is stripped.

Wake port 37371 and injection port 37671 remained bound with connections queued. A bound socket and a live PID are not evidence that their owner is processing requests. The watchdog/heartbeat and RPC processing cannot make progress while their owning threads are blocked inside this lock.

The earlier cursor verification timeout is not the current cause: logs show wake_rearmed at 14:06:57 UTC, subsequent model work, a sent message at 14:07:24 and hook delivery at 14:07:27, before the final Stop at 14:07:49. Do not change wake-unacknowledged retry semantics to fix this deadlock.

## Correction and prevention

Activity inspection now requires the caller's `&ScreenState` snapshot and never acquires the screen lock itself. The duplicate lock-taking helper is removed. All four production callers use one guard, including submit and retry hazard checks. This makes the original no-argument call a compile error and keeps activity and approval checks on the same snapshot.

The Linux unit regression `user_activity_snapshot_progresses_with_queued_screen_writer` observes actual writer queuing with try_read, checks activity from the existing guard, releases it, and verifies the writer completed. A bounded channel timeout makes a regression fail rather than hanging the test runner. The [standalone reproductions](../../.artifacts/humazo-deadlock/reproduction.json) demonstrate the original recursive read timing out and the fixed source-extracted method allowing both threads to finish.

## Verification and recovery

Initial Cargo test compilation was blocked by an unrelated concurrent UI callback type mismatch in Conversation.tsx. The owning source subsequently changed to the matching void callback contract; no UI change was authored for this fix. Focused `just test --bin agentx messaging::delivery::tests -- --test-threads=1` passed all 47 cases, including the queued-writer regression. The standalone old implementation timed out after two seconds, while the fixed implementation exited successfully. Candidate `.bin/agentx-humazo-deadlock-r1` packaged successfully as agentx 0.7.31. SHA256 `eb8090662ef4905f87a55a6b5ffa98e315480d9b3bfec22398fdbdabb9c95930`; native input hashes did not drift during compilation. This uses functional opt-level 0 / LTO false / codegen-units 256, with no optimized-performance claim. See [candidate receipt](../../.artifacts/humazo-deadlock/candidate-receipt.json).

A running deadlocked image cannot adopt source changes. Recovery must use a corrected executable, retire only humazo's failed process/PTY, and resume the existing Provider session `01a1df3a-6393-76a0-ad98-be688b705488`. Preserve the daemon, selected LAN service, other agents, durable mailbox and transcript. Do not replay message submissions as a substitute for reading the existing backlog. Runtime recovery and executable adoption must be recorded separately from source verification.

### Executed target recovery

At 15:16:33 UTC the native `agentx kill humazo --json` retired only the failed target. The corrected candidate resumed humazo headlessly with tmux; readiness was confirmed at 15:16:51. The Provider session ID, workspace, adopted controller role and transcript path remained the same. New Codex PID is 3010673, PTY host PID 3010667; reading `/proc/3010667/exe` proved the loaded image hash exactly matches the corrected candidate. The old Codex was terminated by the native kill operation. Its deadlocked PTY parent did not exit, so PID 2308380 was separately SIGKILLed after checking its exact original executable hash; it is now absent.

The screen RPC now returns promptly with a ready empty composer. Native event #35187 records mailbox hook delivery, #35188 records a new model request, and #35197 records an actual Bash tool invocation. The previous unread backlog is no longer shown. These are execution/delivery observations, not a claim that all queued tasks have finished. No resubmission of the backlog was performed. See [recovery provenance](../../.artifacts/humazo-deadlock/recovery-provenance.json), [events](../../.artifacts/humazo-deadlock/recovered-events.txt) and [post-recovery state](../../.artifacts/humazo-deadlock/after-recovery.json).

The selected :38023 listener and other protected listener records are unchanged; default `.bin/agentx` and the old r1 executable retain their original hashes. This adoption applies to humazo only. Other agents already running older PTY images do not acquire this correction automatically; later lifecycle/release work must adopt a corrected image before claiming deployment-wide prevention. The source API and concurrent regression prevent this diagnosed reentrant-read path in future corrected builds.

### Fresh message round trip

After recovery, diagnostic request #35232 was sent through the native mailbox. humazo replied with ack #35241, “humazo 已恢复收发”. This confirms fresh inbound triggering and outbound messaging after corrected-host adoption, beyond process/readiness observations. The earlier pre-recovery diagnostic #34539 was not answered individually; its request watcher reported idle without reply. The fresh round trip establishes the repaired transport without claiming every historical request received an individual response.
