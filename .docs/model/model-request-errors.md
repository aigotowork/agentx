---
schema: document
status: active
owner: agentx
last_updated: 2026-11-14
description: Hook-observed model request errors, participant state and recovery evidence.
---

# Model request errors

## Scope and authority

An `error` participant has a reported model request failure without observed success afterward. It can still be alive and retrying. No `recovering` status is introduced. This slice installs synchronous Codex PreModelRequest/PostModelRequest hooks only; it neither connects to app-server nor schedules retries. Other providers retain their current behavior until they report the same native observation contract.

`agents/model_request.rs` owns observation/state values; `agents/model_request_store.rs` validates and persists them. Provider parsing stays in the Codex hook adapter. The existing daemon participant service carries observations. A optional JSON `instances.model_request` field (empty text means absent) retains the bounded current state and error metadata, exposed in existing participant JSON. The daemon owns all writes and commits status/events/state atomically. State survives daemon restart; database reset remains a reset.

## State model

| Observation | Effect |
| --- | --- |
| Request started | Record turn and latest progress; keep an unresolved error. |
| Request failed | Enter `error`; keep first `error_since`, update `last_error_at`, reason and consecutive failure count. |
| Request completed successfully | Clear current error, reset failure count and return to `active`, unless a stronger approval/exit state or an error from another source intervened. |
| Request cancelled | Record cancellation; do not claim success or clear an unresolved error. |
| Ordinary prompt, tool activity, Stop or heartbeat | Do not clear an unresolved model error. |
| Process exit | `inactive` takes precedence; retain diagnostic metadata. |
| Approval | `blocked` may take precedence; once approval clears, unresolved error resurfaces. |

A request-success observation is recovery evidence. Generic `active` or `listening` writes are not. `error` is not idle and never authorizes ordinary PTY message injection. Failure timing and progress are structured values for future governance, not instructions to automatically resubmit input. The unresolved error retains its failing turn ID even when a new request starts. `error_since` measures a continuous unresolved error episode; `last_error_at` resets on repeated failures; `last_progress_at` advances on starts and successful completions.

## Identity and ordering

Observations require exact participant incarnation, native session, turn and existing process/attachment authority. Old-session and old-incarnation callbacks cannot affect successors. Synchronous hooks avoid normal pre/post overtaking. The current turn and a 32-entry retired-turn set reject late callbacks for recently replaced turns; arbitrary older start callbacks are not orderable without a native sequence; completion cannot clear an error belonging to another turn. Exact same-request IPC retries use the existing retained request mechanism. Native hooks do not supply a stable per-attempt identifier, so arbitrary delayed callbacks within the same turn cannot be perfectly paired. Do not claim this guarantee or infer final-turn failure from a request failure. Hook failure is logged and never reported as a model success.

## Consumers

Use existing status `error`, context `model:request`, and structured model-request metadata. TUI displays this recoverable request condition in yellow with an explicit error label and compact cause. Other error origins retain red styling. CLI JSON and status events expose the same metadata; error filters work through existing status predicates. Human text must not be parsed for governance. Process liveness, request failure and mailbox receipt remain separate facts.

For live local managed Codex terminals, use [`agentx recover`](../proposals/agent-recover.md) to request one continuation after the failed model turn ends. Recovery admission, input submission and Provider success remain separate facts. See the [original PTY evidence](../../.controller/agent-recover/delivery.delivery.md#verified-linux-outcome) and the [direct terminal follow-up](../codex-activation/acceptance.md#direct-shared-terminal-launch).


## Acceptance

Exercise failure, repeated failure, retry start, success, cancellation, Stop/tool/heartbeat persistence, approval precedence, old session/turn/incarnation, duplicate receipt, transactional rollback and database reopen. Verify actual Codex hook registration/dispatch, JSON/event output and TUI error presentation. Use disposable databases/processes only. No installed executable or live daemon replacement is part of this change.

Implementation entry points are [observation types](../../src/agents/model_request.rs), [transactional projection](../../src/agents/model_request_store.rs), [Codex hooks](../../src/integrations/hooks/codex/client.rs), [query enum](../../src/service/queries.rs) and [TUI presentation](../../src/tui/model.rs). Cancelled or late completed observations do not reactivate an already-listening participant.


## Usage and limits

`agentx list NAME --json` and `agentx list --json` include `model_request`. Its `error` member is null after recovery, otherwise it carries `source`, failing `turn_id`, `reason`, `error_since`, `last_error_at` and `consecutive_failures`; `last_progress_at` is alongside it. Existing `events --status error` filtering/subscription consumes the emitted status records, including the same model-request snapshot. TUI uses yellow `!` for `error` with `model:request`, plus a compact request-error label where context is displayed; unrelated errors remain red. A governance consumer should use fields and event IDs, not terminal colors or free-form descriptions.

Hook reports have a five-second execution timeout and do not change model context or consume mail. A Hook process killed before the daemon accepts its report cannot guarantee delivery; its error is logged rather than converted to success. Existing sessions must obtain the new hook configuration through a new launch/resume. The implementation is locally verified with synthetic Codex hook payloads, not a vendor-model outage test. Only the registered Codex model-request hooks supply this new error source in this slice.
