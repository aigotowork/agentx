---
scope: Project Space release readiness and live message history reads
status: rechecked-after-rollout
owner: kula
related: ["unified-entry.task.md", "../daemon-performance-audit/foreground.task.md"]
---

# Space release readiness

Project `:38023` served the unified Space app on 2026-11-21 local host time. The loaded binary (PID `2209989`, `/proc/2209989/exe`) was AgentX `0.7.31`, build `native-build-v1:811568b8-3451-4305-8e09-3d8c2fbe178b`, SHA-256 `e576006af1b21da486e2c47c8b1cf2ebad9ac93adf693858bcd36111980fd64e`. The document included the native scope and its same-origin bundle. Do not overwrite this listener to deliver the already running unified app. This audit changed no production process, executable, config, data, or user action.

[Live Chromium observation](../../.artifacts/space-domain-sdk/live-audit-20261121/report.json) reached thirteen themes on the same exact conversation and draft. At 390px, chat, scene, composer and Transcript tab access were available. Management and Inspector rendered. The browser allowed only same-origin GET/HEAD and reported no page errors. The overall live check failed because a message history GET returned 503. Rendering does not certify the sibling applications' features.

The [same loaded digest](../../.artifacts/space-domain-sdk/live-binary-replay-20261121/manifest.json) passed [five embedded Chromium journeys](../../.artifacts/space-domain-sdk/live-binary-replay-20261121/browser-report.json) in a disposable native scope: roster and listener binding, theme and layout continuity, native send and reply, original receipt, explicit read-only Transcript, and reopening the same conversation without replay. A freshly built debug candidate also passed the [scoped browser journeys](../../.artifacts/space-domain-sdk/final-r1/browser-report.json). The SDK build, Space type check, 31 focused Node cases and the operation failure script passed. The Provider was a local hook fixture.

The [additional Space actions](../../.artifacts/space-domain-sdk/live-binary-space-actions-r2-20261121/browser-report.json) on the loaded digest passed exact link restoration in a new browser, attachment upload/send/download with byte equality, termination into read-only history, and resume with a new exact incarnation and an isolated draft. The earlier [joint browser run](../../.artifacts/space-domain-sdk/live-binary-lifecycle-20261121/browser/report.json) stopped at an Inspector history selector; that Inspector flow remains unverified. Directory contact, mailbox mutation, fork, browser extension/iframe and real Provider interoperability were not signed off in this audit.

The optimized candidate `.bin/agentx-space-domain-sdk-r1` (SHA-256 `fc0d6a97d389ce72d8a66b7cd83c8ab58ed54db025e1f505e16c8e3a50e40511`) passed the [same five core browser journeys](../../.artifacts/space-domain-sdk/release-candidate-20261121/browser-report.json) in a disposable native scope. The rebuilt binary is a distinct artifact of the current worktree, with native changes outside Space. No representative history volume test or equivalent assurance for those changes was established on this artifact.

## Reproduced release blocker

Three sequential live GETs of the same exact conversation, all with `limit=20`, took approximately 2126, 2392 and 1177 ms. The first two returned `503 Unavailable: Error code 9: interrupted`; the third returned 200 with eight records. Request IDs retained in the scope operational log are `852b196c-3056-4ed2-a6d1-06c429548d11`, `b781defb-39f1-4b64-9686-f049e18a5006` and `eb048211-a659-499e-88b0-d25495971e3c`. The HTTP log confirms the normalized route. This is a finite reproduction, not an error rate.

The native [reader budget](../../src/service/read_application.rs) expires at two seconds or the caller deadline; the [SQLite hook](../../src/db/mod.rs) also caps instructions and returns `SQLITE_INTERRUPT`. The [exact message page](../../src/management/store.rs) delegates to the shared query, which [matches participants](../../src/events/filters.rs) by sender or by expanding the delivered-recipient JSON. That makes the volume of examined rows a plausible driver for sparse histories even at `limit=20`. Distinguish elapsed time, examined work and queue wait in a representative disposable database before changing the query or budget. [Space](../../web/agent-space/src/live/domain-office.ts) keeps the draft on error and requires a successful exact read before sending. This is a primary user journey and blocks sign-off for a replacement.

An [in-memory SQLite probe](../../.artifacts/space-domain-sdk/live-audit-20261121/sparse-history-query.json) kept its result at eight matched messages. With 10,000, 100,000 and 500,000 unrelated messages, the query shape performed roughly 0.28, 2.8 and 14 million VM instructions, respectively. This confirms growth in examined work with unrelated rows in the fixture, without attributing the live interruption to a specific trigger or accessing the live DB.

Before accepting a replacement, exercise a sparse exact conversation at representative history volume, including open, pager continuation and incremental catch-up. Preserve sender and actual recipient visibility, stopped/current time bounds, opaque cursors, attachments and exact identity; keep finite read deadlines. Verify the exact candidate and its shared Management/Inspector paths, then compare its included native changes and rollback identity with the running artifact. A successful small-scope test or release compilation alone does not close the live failure.

Implementation and candidate evidence now continue in [native history repair](message-history-repair.md). The current project listener remains the running pre-repair artifact.

## November 22 post-rollout recheck

The listener statement above describes November 21. On November 22 the [live r5 artifact check](../space-conversation-workbench/delivery.md#2026-11-22-live-status) and [read-only probe](post-rollout-history.py) confirmed both scopes advertise latest history and have complete conversation index coverage. The [project report](../../.artifacts/space-conversation-workbench/post-rollout-audit/history-project.json) includes the retained incarnation of the previously interrupted conversation (its lifetime spans the audit): the legacy read returned the same eight messages in 4 ms and the latest-history route returned eight in 6 ms. One other retained project conversation returned 20 latest and 6 older records in 6 and 4 ms. The [home report](../../.artifacts/space-conversation-workbench/post-rollout-audit/history-home.json) returned 20 latest and 12 older in 8 and 5 ms. Pages were ordered and did not overlap; requests using the live watermark succeeded with no later records in these samples.

Close the reproduced warm-history failure for these sampled paths. The measurements are individual local requests, not a latency distribution. They do not verify incomplete backfill, live incoming messages during an older-page read, or sustained traffic. No data, service, or executables were changed, and the user's pause on large-data testing remains in force.
