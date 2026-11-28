---
schema: task
status: completed
description: Publish v0.7.34 and upgrade the selected local and personal services.
---

The operator requested packaging, upgrade and publication, selected local project/Home and public personal with its uplink hosts, and specified root@192.168.10.3 as the personal deployment host. [Implementation and isolated acceptance](task.md) are the source baseline. [Previous coordinated rollout](../cloud-namespace/local-remote-release.md) identifies the deployment volume, two local Scope ports and dedicated uplinks; confirm current identities before any mutation.

Build from an isolated branch on the current GitHub main, retaining its native archive workflow and published v0.7.33 behavior. The operator corrected the initial release cut: ship the complete current product source, including terminal injection judgment, wake handling, Relay compatibility changes, Namespace v2 and shared Space. Keep private operation evidence, existing tags and the original checkout out of the release commit. The restricted local build was interrupted and never published or installed. Public PR gates precede merging and tagging.

After validating exact release artifacts, stage both rollback executables and the prior cloud image. Update personal's existing v2 volume first, then the local selected daemons and the two dedicated uplinks. Do not rotate credentials, edit unrelated services or restart Agent hosts silently. Verify original generations, listener/installation identities, pending operations and read-only public access. Unknown outcomes do not authorize retries or replacing a failed scope without inspection.

## Release execution

[PR #4](https://github.com/aigotowork/agentx/pull/4) contains the complete v0.7.34 source, including Unix/Windows terminal injection and its controlled fixture. The optimized Linux build passed both legacy migrations, the guarded Codex PTY process with idle/deferred delivery and no repeated unacknowledged wake, and Docker smoke. The exact snapshot's native test subset passed 365 cases and the SDK suite passed 38. The first PR full CI exposed Linux-only symbols in Windows Clippy and assertions expecting the older stale-generation error. Both are corrected on head d3d6f8a; the forged exact reference still expects the separate stale-reference error. Do not merge/tag the superseded earlier heads.

Current PR gates: [CI](https://github.com/aigotowork/agentx/actions/runs/45516031988), [four native archives and old-daemon upgrade](https://github.com/aigotowork/agentx/actions/runs/45516032046), [cloud preflight](https://github.com/aigotowork/agentx/actions/runs/45516031885).

Deployment readback confirmed personal on the original v2 volume with zero pending operations. Both consumers' journals had zero unreported operations. The workstation itself is longser; use the user unit locally. Direct xser01 SSH is unreliable; authenticated ProxyJump through the selected cloud host verified the system unit and exact existing Scope. Use strict known-host verification on both legs. Do not rotate credentials or restart Agent hosts as part of daemon adoption.

At the operator's follow-up, the corrected CI, workflow lint and cloud preflight were green. Apple Silicon, Linux and Windows PR archive builds were green, and Intel Mac was still in Build artifacts before the per-runner upgrade verifier. The Linux GitHub PR archive independently passed checksum, executable version, real v0.7.33 takeover with retained receipt and neighboring Scope, and the Cloud Space native API scenario. Production preflight still showed the original personal image with both enrolled hosts and zero pending operations. The exact-head continuation was alive and had not merged/tagged or begun a rollout.

The operator requested a native self-reminder so the release does not lose its conversation while GitHub runs. Project Scope accepted reminder `rem-7363e5aff6ac4efe9d2bd1de3109db37` for the exact current `boho` Agent, due 2026-11-25 00:53:46 UTC. On wake, verify its receipt, read the current PR/tag and continuation log, continue a stopped rollout from its exact evidence, and schedule another bounded follow-up if external jobs are still running. The detached runner remains the execution owner while gates advance; do not race its merges or live cutover.

The first reminder was enqueued and delivered as event 97975. At readback, PR #4 remained open, no release was published, and the Intel Mac archive job had reached post-build cache handling. Follow-up `rem-defa19f37dcd41bd884f8c558b1a6fd7` was confirmed pending for the same exact Agent, due 2026-11-25 01:04:45 UTC. The CLI discovery follow-up is separate from the frozen release tree.

The second reminder was delivered as event 98099. All four archives had built, and Linux, Windows and Intel Mac verification had passed. Apple Silicon verification was the sole remaining PR job, still queued. The supervisor was alive; PR #4 was still open, the GitHub release did not exist, and no cutover had begun. Follow-up `rem-c35997cf0763476eb6b73c7a38cbf49d` is confirmed pending for the same Agent, due 2026-11-25 01:20:30 UTC.

All three PR workflows passed. Git access recovered through the proxy after a transient TLS failure.

The supervised rerun merged PR #4, verified the resulting tree against the frozen source and pushed the annotated `v0.7.34` tag at `533a12e0add83eb9bf99b51228e2650dcc1e8d86`. The supervisor is waiting on tagged Release and cloud image workflows; tag publication alone is not archive publication or live adoption.

The fourth reminder arrived as event 98165. The exact tagged [Release workflow](https://github.com/aigotowork/agentx/actions/runs/45525545164) was building native archives, and the [cloud image workflow](https://github.com/aigotowork/agentx/actions/runs/45525544976) was exercising Namespace Space HTTP and browser operations. The supervisor remained active; GitHub had not published the release and cutover had not started. Follow-up `rem-80c57ce609ea42d5ba8d8509198e6ad3` is pending for the same Agent, due 2026-11-25 01:48:36 UTC.

The fifth reminder arrived as event 98182. Both tagged workflows passed, all four archives passed their tagged upgrade verifiers, and [GitHub Release v0.7.34](https://github.com/aigotowork/agentx/releases/tag/v0.7.34) was published at 01:57:31 UTC. The supervisor stopped before cutover on an asset download TLS timeout. The download recovered through `proxy`; the local verifier now skips empty manifest lines and checks the four exact native archive names independently of the source archive. Published asset digests and the Linux executable version/architecture were verified.

The tagged Windows CI check timed out once during a workspace-list read. It passed on rerun for the same commit.

Deployment resumed after the original lease renewed and both installation IDs were confirmed online.

## Completed live adoption

The [public release](https://github.com/aigotowork/agentx/releases/tag/v0.7.34) and exact tagged CI, Release and cloud image workflows passed. The published Linux binary SHA-256 is `a78706d22c56f443a1a6a9a9049c815b6881fcfc3b0887152f9b013abfe42732`. The [rollout receipt](../../.artifacts/direct-calls-release-v0.7.34/rollout/completed.txt) confirms adoption on personal at `root@192.168.10.3`, both enrolled uplinks (`longser`, `xser01`), and local Home and project Scopes. Personal uses image `sha256:2d5cf49264ba0fb04468a7bd1f28df56853f0a811aed6f61517b9107e6403c6e` on its original volume. Both worker enrollments and local database generations were preserved.

The transient supervisor's cgroup reaped both newly started local daemons when it exited. The project daemon had already auto-started in the original session; Home was explicitly started there. The final Home and project status receipts, byte comparison with both running executables and live HTTP reads at :38000 and :38023 verified the selected services outside the supervisor. Future supervised rollouts must start local daemons in a lifecycle that outlives the supervisor.

The [final public readback](../../.artifacts/direct-calls-release-v0.7.34/rollout/cloud-final-public.json) verified the original deployment and Namespace generation, both original installation IDs, existing workspaces, and zero pending operations. xser01 briefly recorded `reconnecting (lease_retry)` both before and after cutover; the last public retry recovered without changing its configuration or restarting the worker again. Continuous lease availability was not established. Rollback executables and the previous image are retained; [rollback targets](../../.artifacts/direct-calls-release-v0.7.34/rollout/rollback-targets.txt) records exact locations. Older Agent hosts were not restarted; reopen them individually when needed to adopt the new IPC. The final reminder was canceled after the rollout receipt was recorded and that cancellation survived the project daemon start.
