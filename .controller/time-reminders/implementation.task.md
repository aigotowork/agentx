---
schema: task
status: completed
owner: boho
---

# Native one-shot reminders

[Approved plan](../../.codex/attachments/fc385c54-f63e-4c44-a004-d685e852a09d/approved-plan.md) owns the user-facing contract.

| Boundary | Responsibility |
| --- | --- |
| Events service / reminder use case | Resolve current caller; validate and reconcile exact target and request ID; drive indexed bounded due/recovery passes and state transitions. |
| SQLite reminders adapter | Own additive v30 schema, exact participant storage, indexed lookup and conditional state updates. Use the same write transaction as event append. |
| Messaging / delivery readers | Produce a system direct message with a frozen participant fence; enforce the fence before delivery or pending counts. Historical event observation remains available. |
| Daemon writer | Schedule due checks between requests and during idle. Existing derived index backfills remain idle-only. |
| CLI | Parse and render reminder operations; retain legacy predicate/Relay commands; use the same request ID only when reconciling an uncertain create. |

Resolve the overlapping identity wording in the plan by requiring an existing valid native Agent caller whether or not `--for` is supplied; the target selector does not impersonate its target. Recovery re-wakes recently enqueued and still unread events for the exact target; a wake is advisory and never appends a second event.

The additive migration preserves ordinary subscriptions and internal KV records. Managed reset keeps its existing full Scope semantics and also removes reminders. History listing uses bounded keyset pages over immutable due time and ID so retained results do not grow a single wire frame. Automatic Relay push omits the reminder journal entry; the Scope's existing operator event history remains available.

Verification entry points: [native scheduling, persistence and fence cases](../../src/events/reminders/tests.rs), [writer queue case](../../src/daemon/server.rs), [unknown-result reconciliation](../../src/cli/commands/events_client_tests.rs), [epoch compatibility](../../src/daemon/compatibility_tests.rs) and [independent CLI runner](../../tests/time_reminders/README.md).

Linux native verification passed 99 targeted Rust cases: 26 for the reminder contracts, IPC epoch 3↔4, mailbox indexes, conversation history and help; 73 for existing subscription matching, request watches, inbox claims, unread projections and Events host behavior. The dropped-reply case used a real native create with a one-reply transport drop. The writer queue case had both requests enqueued before dispatch, so an idle timeout could not account for the observed delivery.

`just package` passed and checked project [`.bin/agentx`](../../.bin/agentx) as `agentx 0.7.31`. Package SHA-256: `4ed027fb46ec42e11e875a4de8a8b0341a449e1498f643c17dbf1bd8c43b92f4`. The [isolated CLI runner](../../tests/time_reminders/README.md) passed against those bytes: quiet-window enqueue lag was 36 ms; enqueue under sustained native requests was 21 ms (426 replies near the deadline, maximum 11 ms reply gap). It also verified overdue restart, same-name replacement and stop failures, ordinary subscription behavior, authorization, cancel and retained history.

The fixed candidate passed the native multi-process compatibility, restart/reset fence and archive continuation cases. An additional ordinary-message lost-reply regression first stopped at a stale fixture argument, `send --name`, before exercising delivery. The fixture now uses the current `--iam` selector; it does not change the executable.

The ordinary-message lost-reply case passed on rerun against the unchanged package, bringing the targeted multi-process checks to 4 passing cases. All verification is Linux local/disposable Scope evidence. The project binary was built, but no installed program was replaced and no existing daemon or Agent host was restarted.

Live self-test follow-up: the user requested a five-minute reminder for the actual current `boho` session. The exact candidate's ordinary CLI create against the session-selected project Scope was rejected during Hello (client epoch 4, live daemon PID 2703841 epoch 3), before a reminder was created. The executable loaded by the current Codex host (PID 2553640) still handshakes with the epoch-3 daemon and differs from the candidate. A daemon-only upgrade would therefore not establish wake delivery to the current session. The shared Scope, other live Agents and port 38023 were left untouched; an isolated Provider session would be a separate test.
