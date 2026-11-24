---
schema: task
status: completed
owner: boho
---

# Native one-shot reminders

[Events CLI usage](../../.docs/architecture/event-subscriptions.md#一次性时间提醒) owns the user-facing contract.

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

Prior local evidence includes 99 targeted Rust tests, four daemon process cases, the independent CLI acceptance, and two real Codex self-reminders that started a new turn without polling or input injection. The latter observed 51 ms and 53 ms from due time to durable enqueue; those measurements do not include Provider wake latency. Release source/build checks and current daemon adoption are separate.
