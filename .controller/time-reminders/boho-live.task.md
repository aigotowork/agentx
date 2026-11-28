---
schema: task
status: completed
owner: boho
---

# Live five-minute self-reminder

The user approved the temporary selected-scope transition after the native create for current `boho` hit the epoch-3 daemon. The original native Codex thread is `01a20cf1-b9cf-7742-b08e-ee5f8c8d56b5`. The detached helper waits for the originating turn to finish, retires only that exact Provider host, uses the native selected-scope restart, then resumes the same thread on the pinned epoch-4 candidate. The resumed Agent schedules its own five-minute reminder and ends its turn; no tool or manual injection waits on its behalf.

Acceptance requires the same transcript to show the scheduling turn completed before the deadline, real system reminder context after the deadline and a completed reaction turn, plus the retained native event ID. The helper restores the original daemon and resumes the same thread on epoch 3, even if the reminder test fails. Other live Agent hosts are not terminated. No database is reset or restored. A disposable rollback rehearsal verified the old executable can serve a v30 database without archiving it or changing its generation.

Runtime evidence: [operator receipt](../../.artifacts/time-reminders-boho-live/receipt.json), [handoff instructions](../../.artifacts/time-reminders-boho-live/handoff.md) and [loaded-image baseline](../../.artifacts/time-reminders-boho-live/baseline.json). These are specific to this live session. The [implementation task](implementation.task.md) owns the separate package and isolated verification.

The operator receipt reached `complete`: the reminder was enqueued as event 95849, 51 ms after its deadline, and the original Codex transcript independently confirmed the Provider was woken and completed a response. The original epoch-3 daemon build was restored with the same database generation, the same Codex thread was resumed, and all 10 other Agent incarnations were retained. This was a temporary live test, not an epoch-4 rollout.
