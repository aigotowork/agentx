# Mailbox claims and activation

Read [source ownership](../README.md), [attachment identity](../agents/AGENTS.md), the current [daemon contract handoff](../../.controller/domain-daemon/contracts-handoff.md), and [consistency Task](../../.controller/domain-daemon/tasks/consistency.md). Shared-runtime Delivery records are historical evidence.

| Responsibility | Source of truth |
| --- | --- |
| Context bundle create/lookup and bounded event projections | [bundle_service.rs](bundle_service.rs), [bundle_store.rs](bundle_store.rs); CLI bundle owns file/transcript reads. Semantic validation stays in bundles.rs; explicit-cwd file warnings run on the host only. |
| Durable submission, replay/readback and native inbox operations | [service.rs](service.rs), [service_store.rs](service_store.rs); [submission.rs](submission.rs) owns routing/persistence, [service_remote.rs](service_remote.rs) adapts `PendingInbox` over IPC. |
| PTY durable observations and cleanup | [delivery_service.rs](delivery_service.rs) and [delivery_store.rs](delivery_store.rs); host timing, scraping and terminal I/O remain in [delivery](delivery/mod.rs). |
| Canonical message formatting values | [formatting.rs](formatting.rs) and pure [tips.rs](tips.rs); explicit invocation/participant hints are frozen with the claim. |
| Claim, phase, evidence and recovery contracts | [inbox.rs](inbox.rs) |
| SQLite transactions and frozen output preparation | [inbox_store.rs](inbox_store.rs) |
| Provider-neutral offer/coalescing/event policy | [activation.rs](activation.rs) |
| Durable schema and deletion retention | [database context](../db/AGENTS.md) |
| Hook output adapter | [output.rs](../integrations/hooks/output.rs) consumes frozen output and exact receipts; `DeliveryAck` in [hooks/mod.rs](../integrations/hooks/mod.rs) remains a host adapter. |
| Explicit listen and inline output | [CLI context](../cli/context.rs) and [listen](../cli/commands/listen.rs) |
| Independent executable persistence cases | [acceptance_tests.rs](inbox/acceptance_tests.rs) |

The PTY host keeps an unacknowledged Claude/Codex wake latched instead of interpreting the ten-second verification timeout as rejection. For Codex, uncertain rendering, changed prompt ownership and exhausted Enter retries also latch the already-written wake. Notification arrival, pending mail and elapsed time do not rearm it; mailbox cursor progress or an empty pending snapshot do. The host continues heartbeat and endpoint maintenance while paused. This is a host-local duplicate-wake guard, not proof of message delivery or a durable provider compaction state; process restart resets it. Keep other providers' retry policies separate.

Explicit model-error recovery uses a [separate ticket](../agents/recovery.rs) on the managed Codex delivery loop. It uses the same prompt safety checks and verifies ownership before its single Enter; it never advances the mailbox cursor or bypasses an unresolved ordinary wake.

For managed participants every consumer contends on the same persistent claim. Native-input mode suppresses hook consumption; explicit listen may compete. Preparation freezes eligible messages, bounded window, formatted context, consumer and correlation id under SQLite serialization. No unrelated global event watermark may become a receipt frontier.

Persist Issuing before external output/input. Confirm only after the exact consumer's boundary evidence: successful hook flush, successful explicit output flush, or an adapter-verified recorded provider input. Admission, turn completion, cancellation and a missing response never confirm delivery. Hook preparation conservatively marks Issuing before handing its result to dispatch, so a dropped result may require explicit recovery.

Message submission normalizes caller intent before durable lookup and freezes recipients only on first acceptance. Actor/input/outcome, event, inline bundle, thread membership and request watches commit together; replay returns the retained recipient set without repeating external wake effects. Legacy output uses retained `OutputToken` plus exact participant incarnation for confirmation. A global event watermark or uncorrelated cursor is never output evidence.

[Legacy Codex PTY output](service_store.rs) also reserves a nonempty batch in the preparation transaction, across both hook callbacks and explicit reads. The reservation follows the authenticated output process and ends after exact confirmation or confirmed process exit/replacement. If the process exits without a confirmation, unread messages remain eligible for retry. A crash after flush and before confirmation can still cause a retry; this path has no provider receipt to resolve that ambiguity.

`PreparedOutput` supplies remaining count and the last eligible backlog event from the same preparation transaction as its bounded output window. These values support client pagination and late-arrival hints; confirmation still uses only the exact retained output token or managed batch receipt.

Unmanaged output also carries `formatted_context` frozen with its token. Managed and unmanaged formatting use the same inbox-store formatter, explicit scoped hints and borrowed database records; clients do not reread the roster to reconstruct the prepared hook text. `ConfirmOutputAndStatus { receipt, announce }` confirms an exact unmanaged token or managed written-output receipt and applies canonical delivery status atomically. Sender display and message timestamp come from the preparation snapshot; a retained per-output marker prevents replay from overwriting later activity. The `announce` flag controls name announcement, separately from delivery status. Call this operation only after successful output write and flush.

Native message persistence still calls `submission::resolve_reply_to_local` through `persist_message`; include that exact helper in its borrowed-storage closure. The legacy `get_thread_from_event` wrapper and `bundles::{validate_extends_reference,create_bundle_event}` are test-only after the actual send/bundle cutover. Native thread lookup remains fallible `try_get_thread_from_event`; native bundle event persistence belongs to bundle_store and the message store's inline transaction.

Normal confirmation requires the current row/execution/binding and exact batch/window; cursor/claim confirmation is atomic and forward-only. A retired or replaced binding's receipt can be reconciled only through `reconcile_predecessor`, validating the same durable session lineage and current successor independently. No-input evidence is privileged adapter/recovery evidence; the store rejects mismatched attempt, empty history reference and NeverIssued after issuing. It does not independently query provider history or infer a negative result from timeouts.

`ActivationPolicy::prepare_request` and `finish_admission` support a bridge that owns both inbox and driver without aliasing mutable ports. Construct the policy with `Default`; `observe` persists fenced attempt progress, while delivery confirmation remains a separate evidence operation. Hosts own notification/scan scheduling and provider I/O.

Observation insertion is selective and advisory. Successful Submit supplies MessageAccepted with its exact durable acceptance request/event/generation/replay result in addition to generic EventAppended. A lost reply still reconciles through native Acceptance; telemetry never authorizes replay. Successful inbox phase/confirmation and unmanaged output callbacks freeze only the safe native claim/participant/boundary projection, defer through the existing outer transaction and discard on rollback. Fresh output observation UUIDs have no reverse mapping to native receipt tokens; omit full receipts/frontiers/provider turns and message text. See the [Events mapping](../../.controller/observability/implementation-tasks/events.md#composition-mapping-and-lifecycle-handoff), [postcommit guide](../db/AGENTS.md) and [composition handoff](../../.controller/observability/implementation-tasks/composition.md).

Accepted submissions retain `scene_addressing` alongside existing routing facts: exact local participant refs are frozen under the send transaction; external bigboss authority and canonical resolved human recipient are explicit booleans. This read-only metadata neither changes routing nor establishes delivery/read. Legacy messages without exact refs are not rebound from names by the office consumer. See [office implementation](../../.controller/office-details/implementation.md).

For the diagnosed humazo PTY screen-lock deadlock, correction, regression evidence and recovery status, read [the incident record](../../.controller/domain-daemon/humazo-deadlock.md). Screen activity checks must consume an existing snapshot; never recursively acquire the screen RwLock while holding a guard.

For daemon saturation caused by repeated PTY snapshots, follow [PTY poll pressure](../../.controller/pty-poll-pressure.task.md). Screen rendering stays local; a fresh exact participant check precedes Enter. Use the snapshot's cursor for bounded mailbox reads and restore missing endpoints using the observed ports. Snapshot failures back off even under repeated wakes.


Routing consumes `routing::InstanceInfo { name }`: local targets match exact native names, remote targets match exact or unique device-qualified prefixes, and the human virtual identity retains its existing device-independent behavior. Formatting and wake previews show the original native sender/recipient names. Per-instance hint lookup remains independent of identity formatting. Exact participant/incarnation scene references, thread membership, generation and mailbox receipt fences retain their existing ownership.

[delivery_report.rs](delivery_report.rs) owns read-only accepted-event delivery facts using frozen exact recipients. `service::Request::DeliveryReport` never advances a cursor or chooses a same-name successor. [Repair usage](../../.controller/message-delivery/usage.md) explains CLI lookup, last authenticated PTY snapshot progress, unknown legacy evidence and explicit recovery limits.
