---
schema: task
status: first-batch-verified-in-isolation
description: Reduce write-queue occupancy and repeated reads identified by the workload audit.
links: ["audit.md", "workload.json", "../daemon-io-isolation.task.md"]
---

1. Message submission owns one routed plan under the existing write transaction. Validate/derive recipients before attaching generated bundle display text. Persist the same plan and retain the same acceptance, recipient order, exact instances and replay contract. Use narrow participant reference lookup for scene facts.
2. `list --names` selects only current names from an independent read snapshot, filters workspace and labels using the current rules, and has no cleanup side effects. The new native query is advertised explicitly; a client that acquired an older daemon keeps the old typed listing path. Stopped history and detailed listing have separate behavior.
3. Thread lookup must seek by thread without scanning unrelated KV or loading all Agent names. Keep existing durable thread state and add a targeted index in the current subscription-index rollout. The query must preserve ordering and skip non-current member names on both paths.
4. Stop routine Web reads while hidden and refresh immediately on return; keep explicit actions available. Defer new durable UI revisions, exact unread projections and removal of existing indexes until workload evidence justifies their own contracts.

Verify on disposable SQLite and native/CLI fixtures. Existing performance probe owns design comparisons; measure the actual final queries separately. Do not adopt a candidate or restart an existing daemon as part of these source changes.

The lightweight output shows the current committed roster. Until the existing maintenance path retires them, exited participants may remain in that view; `--names` is not a process liveness test. The detailed and stopped commands keep their existing behavior. Thread lookup retains historical KV representation and rollout fallback, and the typed query never uses a SQLite client fallback.

Linux source verification: 73 focused native tests passed, covering message admission, frozen recipients, replay, rollback, bundle display text, indexed and legacy thread order, independent WAL names lookup with workspace/labels, and capability negotiation against old and new Ready frames. Six Web tests passed for the Space production consumer and Management foreground scheduler; both project TypeScript checks passed. [Workload evidence](workload.json) now also checks the exact implemented thread query: with 272,000 unrelated KV entries it uses the thread expression index and the instance primary key, returning the same two live members in order with fewer than 100 SQLite VM instructions versus about 1,088,000 before. Fresh index build cost and production p95 are not part of this experiment. This batch did not deploy or replace any runtime.

Next, measure foreground polling on the verified combined candidate and decide on lightweight per-view change marks. Audit exact-detail and full-roster hydration separately from the names path; retain its explicit liveness semantics when removing query-time maintenance. Use the measured sparse/broadcast mix to decide whether to build the accepted-recipient count projection. The stopped-history partial index remains a candidate until its actual plan, index build and write cost are verified on representative data.
