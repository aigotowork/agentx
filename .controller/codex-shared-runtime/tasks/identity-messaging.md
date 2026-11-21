---
schema: task
status: completed
owner: reko
description: Implement provider-neutral identity attachments, exclusive delivery claims and activation/recovery policy.
links: [../AGENTS.md, ../delivery.md, ../../../.docs/proposals/codex-shared-runtime.md, ../../../.docs/codex-activation/README.md, ../../../.docs/codex-activation/protocol.md, ../../../.docs/codex-activation/session-environment.md]
---

# Assignment

## Execution state

- Runtime claim: reko; identity/persistence/messaging only. No delegated workers and no routine test/build execution.
- Current phase: local assignment completed; #1324 was diagnosed and handed to bridge owner vume, who confirmed the callsite fix in #1383. No generic transaction change is required for that defect.
- Method: model the persisted identities and effect boundaries first, retain existing non-Codex paths, and provide meaningful cases to the integration owner.

### Current-to-target model

| Fact / boundary | Current authoritative source | Required change |
| --- | --- | --- |
| Participant incarnation | `src/db/instances.rs::InstanceRow.created_at` and existing hook ack fencing | Preserve the exact `f64::to_bits()` identity; a reused name is not the same participant. |
| Process/session identity | `src/agents/identity.rs::resolve_identity_with_expectation`, `src/agents/binding.rs` | Shared execution cannot select one participant. A known provider session resolves its namespaced attachment before any process fallback; stale/ambiguous known sessions fail closed. |
| Persistence | `src/db/mod.rs::with_immediate_transaction`, `with_write_scope`; schema v20 | Reuse SQLite immediate write serialization. Persist attachment lineage and claims independently of cascading live instance deletion. |
| Queue observation | `src/db/events.rs::try_get_unread_messages` versus `get_unread_messages` | New contracts propagate query failure. Snapshot `through` is the last eligible message, never a global event watermark. |
| Hook consumption | `src/integrations/hooks/common.rs::prepare_pending_messages`, `commit_delivery_ack` | Attached participants use the same exclusive claim as native/listen. Successful output flush is hook evidence; no output or blocked hook is not delivery. |
| Explicit/inline receive | `src/cli/context.rs::InlineBatch` and `src/cli/commands/listen.rs` | Join the same claim boundary for attached participants, freeze raw/format content, record the issuing boundary before writes, confirm only after flush. Keep unrelated providers on their existing path. |
| Native effect | `src/integrations/hooks/codex.rs` currently has hook-only delivery | Native admission is separate from input recorded. Persist unknown outcomes and require authoritative proof for release. |

Required invariants: one live owner per provider session; fresh BindingId on every reattach; no stale receipt changes a replacement participant; one unresolved claim per session/mailbox lineage; consumer/evidence agreement; immutable batch/window/content; cursor advances with MAX only to the claimed eligible window; old unresolved claims survive live row deletion and block successor preparation. Timeout is not a no-input proof.

### Integration cases to author

1. Two roots in one execution and two proxy owners: distinct participant/context, duplicate live ownership rejected, closing one owner leaves the other usable.
2. Concurrent Hook / NativeInput / ExplicitListen preparation: one winner; NativeInput suppresses hook consumption; loser cannot advance cursor.
3. Frozen bounded window with irrelevant/global events and a later arrival: receipt advances only through the last claimed eligible message; later mail remains pending.
4. Query error versus empty queue: unavailable never emits an activation or confirms delivery.
5. Delayed output receipt after detach/reattach or name reuse: stale fence rejected; late successful old claim needs explicit predecessor reconciliation.
6. Issuing/admitted crash: retained claim survives reopen and instance deletion; timeout/new execution does not unlock a duplicate native submit.
7. Input item arrives before RPC response: exact batch can confirm once; RPC success alone cannot confirm; repeated receipt is idempotent.
8. Blocked hook/native input: no cursor movement; retry only with relevant no-input proof, and NeverIssued rejected once issuing has begun.
9. Non-Codex and unattached Codex legacy paths preserve existing identity resolution and delivery behavior.

### Open contract questions

- kana released production declarations in #719. Concrete feedback from #447/#554 is incorporated: per-proxy owner, canonical lookup, explicit consumers/evidence and persistent phase changes. The unregistered-resume gap was resolved in #846 by optional provisional identity, now implemented in both generic and native intents.
- Existing `instances.session_id UNIQUE` and raw-key `session_bindings` cannot supply the provider/home namespace. New attachments own that namespace without changing the legacy uniqueness contract. Identity lineage and claim records must survive `instances` deletion without a cascading FK.
- lobo received the case table in reply #618 and the callable production-store handoff. Integration owns independent executable cases and execution.

### Transaction design constraints

| Operation | SQLite write scope and acceptance rule |
| --- | --- |
| Attach/reattach | Validate the exact participant incarnation and canonical session identity, reject another live owner, replace BindingId atomically, and retain old claim lineage. |
| Snapshot/prepare | Validate participant plus active attachment; under BEGIN IMMEDIATE, re-read cursor and bounded eligible messages, reject changed snapshot or unresolved predecessor/consumer, freeze the batch and consumer before returning. |
| Begin external output/input | Persist Issuing before the external write; a crash after this commit retains possible-effect state. No DB transaction is held across external I/O. |
| Confirmation | Under BEGIN IMMEDIATE, match participant/execution/BindingId, batch/window and evidence/consumer; advance the exact live cursor with MAX and commit the claim receipt atomically. Repetition is idempotent, and stale normal receipts cannot take over a successor. |
| Known non-delivery | Release only a permitted phase with matched proof; NeverIssued requires Prepared. An issued attempt remains blocked/uncertain until sufficient provider proof excludes the input effect. |
| Predecessor recovery | Validate canonical session/mailbox lineage and the current successor fence independently of the stale normal receipt; recorded evidence advances only the intended lineage and proven non-input unlocks a new claim. Preserve uncertainty otherwise. |
| Disconnect | Retire only attachments belonging to the proxy owner. Retain session identity and claims, and leave shared execution lifetime with the native service owner. |

Implementation structure: public contracts remain in `agents/attachments.rs` and `messaging/{inbox,activation}.rs`; adjacent private SQLite implementation leaves own transactions, while `db/shared_runtime.rs` owns additive schema v21. Persistent session records retain logical identity, current row snapshot and cursor; attachments retain historical BindingIds and exact owner PID/incarnation; claims are never deleted with instance rows. Canonical resume restores absent rows with a fresh incarnation and rejects a reused name. Owner takeover requires explicit retirement or OS proof that the old owner process ended; no lease timeout is used. Shared Codex context uses `AGENTX_CODEX_SHARED=1`, backend `AGENTX_PROCESS_ID`, canonical `CODEX_HOME`, and native thread id; the Codex adapter supplies provider mapping, while generic store meaning stays opaque.

### Callable implementation handoff

- `SqliteAttachmentStore::new(&AgentxDb)` and `SqlitePendingInbox::new(&AgentxDb)` implement every released trait branch. Public contracts reexport the adjacent implementation leaves. Scoped identity is separate from legacy raw session columns.
- `ActivationPolicy::{prepare_request,finish_admission,observe}` provides sequential composition to vume's bridge, which owns the same injected inbox and driver. `Default` constructs the policy; unused convenience constructors/combined offering were removed after composition selected the sequential seam.
- Generic hook preparation/ack and `InlineBatch`/listen now contend for managed claims. Legacy acknowledgments cannot bypass a managed mailbox. Output errors retain uncertain claims; query failures propagate on explicit receive. Native-input hooks defer mailbox consumption.
- `cli/context.rs` invokes the released Codex mapping adapter before legacy identity resolution. With vume's explicit release (#1014), `start.rs` has a recognized-shared guard preserving canonical identity/bootstrap and forbidding process-default rebind/orphan paths. vume owns remaining launch-name ancestry and fresh-child environment marker scrubbing (#1072).
- Successful DB deletion/finalization saves managed snapshot/cursor and retires exact-incarnation attachments inside the stop CAS, preserving unresolved claims and allowing a legitimate stopped-row resume while the old proxy still exists.
- Current context is discoverable in `src/{agents,messaging,db}/AGENTS.md`. lobo's independent real-SQLite cases live at `src/messaging/inbox/acceptance_tests.rs`; lobo has the callable store handoff and the requested restore/first-resume proof cases. No tests/builds have been run by reko.

Owned-source formatting completed once with scoped `rustfmt --edition 2024 --config skip_children=true`; this is parser/formatting evidence only, not compilation or behavioral acceptance. No owned subprocesses, temporary services or daemon resources were created.

Completion evidence: lobo reported 39 independent acceptance tests passed, including 11 real SQLite cases, in message #1210 and `.agentx/.tmp/deliveries/codex-shared-runtime/acceptance-second.log`. The newly added v20 migration and three policy tests had not yet run in that report. This is independent scoped evidence, not full Delivery/native-client acceptance. reko ran no tests/builds. The reported owned dead-code warnings were resolved by using `owned` in `retire_owner` and removing unused policy convenience entries; lobo owns final checks.

Consumer inspection confirms `bridge/driver.rs` uses `prepare_request`, `finish_admission` and `observe`; `bridge/session.rs` uses `retire_owner`. vume implemented shared launch ancestry and fresh-child marker removal in the existing launch and identity-variable boundaries. Required production store/claim/policy branches are implemented; external provider/history proof remains the provider adapter's authority as declared in the contracts.

All owned source write claims are released. No delegated agents, temporary services, daemon processes, installation changes or experimental resources were created. Any further concrete acceptance defect returns to this task; full Delivery acceptance remains zomi/lobo's responsibility.

### Real-native reservation defect #1324

Observed: native initialize succeeds, but the first `thread/start` exits with `Cannot reserve existing AgentX participant <generated name>` before a root result. Evidence: `.agentx/.tmp/deliveries/codex-shared-runtime/production-mountfix/csr-peqhdh6r/{roots.stderr,result.json,trace.jsonl}`.

Root cause: `bridge/session.rs::reserve_participant` calls `names::generate_unique_name`, which intentionally creates a pending/new placeholder, then rejects any existing instance row. Every generated root therefore rejects its own reservation. The generic helper's documented reservation semantics are correct and must remain intact for other providers. Minimal correction belongs at the bridge callsite: generated placeholders are promoted in place; an explicitly requested existing identity remains subject to conflict/ownership checks. vume owns this exact function; direct write coordination was requested to avoid concurrent edits. Independent native rerun remains lobo's responsibility.

Disposition: vume confirmed callsite fix ownership in #1383, including lock-order inspection. zomi confirmed in #1376 that reko should retain the completed identity assignment and avoid parallel edits/checks. No source behavior change or test/build execution was performed by reko for this failure; generic helper semantics remain intact.

Independent follow-up #1484: lobo verified the callsite fix with real native `roots` and `shared-eof` scopes under `production-rootfix`. Two entries shared one managed backend; a sibling survived proxy EOF and completed another loopback turn. Both scopes cleaned successfully. Seven policy/migration/resume/connected-degradation checks and forty-five direct-stdio/acquisition/send/backpressure checks passed. Idle/busy mail, real parent-child and actual Paseo cases were still in progress in that report; these results do not establish full Delivery acceptance. No reko test/build execution or source writes were added.

Candidate-build follow-up #1768: normal-bin compilation reported `PendingInbox::delivered_through` unused outside tests. Retained the released read-only cursor observation contract with a method-scoped, documented `cfg_attr(not(test), allow(dead_code))`; the production bridge uses `snapshot` instead. This changes no behavior or consumer contract. No test/build execution by reko; final all-target checks remain lobo's responsibility. The tiny source write is released.

Coverage finding: the existing independent bridge helper seeds a root attachment directly, bypassing fresh native root preparation. lobo received the concrete additional case: invoke `handle_outer(thread/start)` on an empty fresh bridge and assert its reserved placeholder is promoted and the native request is forwarded; separately retain rejection of an explicitly requested live name. No additional generic behavior change is needed for the allocator contract.

You are the identity, persistence and messaging professional. Build the real transactional capability needed to bind multiple Codex roots to distinct AgentX participants in one backend, prepare one message batch, normalize delivery evidence and recover uncertain outcomes without losing or double-consuming messages. Generic agents/messaging must not depend on Codex DTOs, home rules or daemon management.

Immediately recover current process/session binding, instance incarnation, mailbox cursor, hook/listen consumption and activation behavior. Establish current-to-target invariants and reject false equivalences such as query error equals empty queue, RPC success equals delivery, or process identity equals one participant. Existing non-Codex launch, identity and delivery behavior must be retained.

## Outputs and ownership

Own the identity/DB/message behavior leaves and exact transactional design assigned by architecture in `../contracts-handoff.md`. Expected scope is generic attachments and claim/activation policy with necessary existing identity, database and delivery boundaries; public types and exact module ownership come from the architecture handoff. Coordinate actual shared hook changes with bridge-composition rather than editing the same handler concurrently.

Keep canonical state/transaction meaning discoverable in source contracts and a narrowly owned adjacent design if needed. The architecture owner receives updates required in the shared protocol/README. Author meaningful normal, stale-binding, concurrency, cursor, blocked-hook and unknown-effect cases for integration; do not write tests that only mirror implementation.

## Start and acceptance conditions

Discovery/model work can start now. Behavior implementation starts once the relevant production contracts are published. Completion requires callable transactions and activation policy consumed by the bridge, consistent old-path handling, explicit recoverability/error semantics, updated owned context, and no unimplemented required branch. The integration owner executes tests and build checks. No further delegation; claim the actual name in metadata and send a usable handoff/completion with evidence limits.
