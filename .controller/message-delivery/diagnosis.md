---
schema: document
status: active
owner: mara
links: [repair.task.md, ../../src/messaging/delivery/host.rs, ../../src/daemon/transport.rs]
---

# Causal investigation

The source transport is not a long-lived daemon subscription: each native service call opens an authenticated local IPC connection and performs Hello, then one operation. Durable messages remain in SQLite until receipt confirmation. Connect-and-close localhost TCP hints wake process-owned NotifyServer loops; a 30-second idle poll recovers missed hints. The PTY delivery thread polls snapshots, gates on Provider state, submits a wake through its local injection server and waits for hook/cursor confirmation. Wake acceptance alone never confirms consumption. A compatible daemon restart reconnects on the next call with the retained database generation; an incompatible protocol correctly refuses Hello.

## Original evidence

Read-only `.agentx/.tmp/logs/agentx.log` contains correlated `delivery.snapshot_failed` for all four original incident identities. In the retained log slice 2026-11-16T21:55:57Z–21:56:32Z: hobo=139, lavu=138, luta=139, zulo=139 failures. All identify protocol2 client cohort `232ed85e-7f99-4bdf-8355-e43a29491b74` against authenticated daemon PID3400255 protocol3 cohort `95d9a9e7-6de0-40f2-95e3-6f9a29f827c7`. This establishes that their delivery loops cannot read backlog after that transition. It does not yet establish the cause of earlier failures or justify weakening protocol fences. Original processes/mailboxes are unchanged.

## Hypotheses and discriminators

| Hypothesis | Observation / next discriminator |
| --- | --- |
| A broken persistent socket stops distribution | Source disproves a persistent socket dependency; each call reconnects. Check successful Poll and stage logs around earlier incident. |
| Old hosts cannot consume after epoch change | Supported by exact original host log failures; expected safety refusal, not proof of an ordinary same-epoch reliability defect. |
| Missing wake hint loses durable work | Idle polling should recover; test missed notification with waiting host and ordinary send. |
| Codex stays active after a completed turn and gate never recovers | Inspect Stop/Interrupt and transcript/activity semantics; quiet output alone must not permit duplicate submission. |
| Confirmation stalls after uncertain wake | Inspect outstanding wake state and native hook evidence; never blindly retype. |

CBM index access failed after its own 30-second connection timeout; targeted source reads are the fallback. The selected correction and executed evidence below supersede this initial investigation checkpoint.

## Selected bounded correction (nezi #44236)

Keep incompatible hosts fail-closed. Add a read-only, event-correlated delivery report to messaging and the ordinary send caller, preserving acceptance exit semantics and exact frozen recipient identities. Report accepted/pending versus mailbox-confirmed separately from Provider execution. Track authenticated PTY snapshot progress independently of hook/model status, with protocol and timestamp; old registrations lack evidence and must remain unknown, never inferred compatible from a listening socket or live PID. Expose report lookup without resending the message. Report stalled/unobserved automatic delivery with a supported explicit `term`/`kill`/`resume` recovery sequence; do not automatically kill, replay or inject. A candidate cannot hot-repair the old live image.

Structure: `messaging/delivery_report.rs` owns typed report and read-only event/recipient projection; `runtime/process_host_store.rs` owns authenticated last-snapshot progress; `messaging/service.rs` exposes an additive report operation; CLI send renders current report and supports event-only status lookup. No existing required wire fields/variants change meaning. Earlier peers do not receive the new response except when explicitly invoking the new operation, which fails explicitly on an older decoder. Existing acceptance/replay and exact receipt operations remain unchanged.

Verification scenarios: ordinary send to a waiting actual controlled Provider after completed turn; queued while active then idle; missed wake recovered from durable backlog; compatible daemon restart preserves exact process and backlog; report does not rebind old recipient to same-name successor; lost/failed flush cannot become confirmation; unknown legacy progress is reported explicitly. Use the existing disposable launch/PTY fixture, with owned cleanup and a separately packaged candidate. No paid model or manual terminal input.

## Actual caller-path fixture correction

The first candidate execution stopped at parent fixture setup: bare `agentx start` in an identity-free isolated operator environment returned `identity required: no AgentX participant in caller context`, before any tool launch or send. Both controlled cases share this same setup, so this is not evidence of a wake failure. The fixture now starts its own disposable daemon explicitly and uses its existing typed `Register` helper for an adhoc parent, then tests ordinary CLI launch/send and the actual child hooks. It does not bypass the repaired delivery path. Retained failing executions: `.artifacts/message-delivery/codex-r1.log` and `qoder-r1.log`. No production start semantics are changed by this repair.

## Current source sequence and recovery boundaries

```mermaid
sequenceDiagram
    participant CLI as send caller
    participant D as scoped daemon / SQLite
    participant H as registered PTY host
    participant P as Provider and hooks
    CLI->>D: Hello + Submit(stable request ID)
    D->>D: Commit event, frozen recipients, acceptance
    D-->>H: Best-effort TCP connect-close hint
    D-->>CLI: Accepted event (not consumed)
    CLI->>D: DeliveryReport(event), read only
    H->>D: New Hello + Poll (also after idle timeout)
    alt protocol matches and generation retained
        D-->>H: Exact actor + pending snapshot
        H->>H: Gate and outstanding-wake state
        H->>P: Local input only when safe
        P->>D: Hook prepares exact output claim
        P->>P: Write and flush context
        P->>D: Confirm exact output receipt
        D->>D: Advance mailbox cursor
    else protocol incompatible
        D-->>H: Refuse Hello before business decode
        Note over D,H: Pending mailbox retained; hint/socket cannot repair loaded code
    end
```

| Stage | Owner / source | Recovery and evidence |
| --- | --- | --- |
| Acceptance | `messaging/service_store.rs`, `submission.rs` | Stable request replay returns retained recipients/event; missing reply is reconciled, not resubmitted. |
| Wake | `db/post_commit.rs`, `runtime/notify` | Connect-close hints contain no mailbox receipt. Idle polling reads durable backlog even without a hint. |
| Host read | `messaging/delivery/host.rs`, `daemon/transport.rs` | One-shot reconnect per call; compatible daemon instance may change, retained database generation may not silently change. |
| Provider input | PTY host state machine / local injection server | Gate safety and uncertain submitted wake remain distinct; timeout never authorizes duplicate Codex input. |
| Consumption | `messaging/inbox_store.rs`, hook output client | Only exact flush/native-input evidence confirms; cursor progress does not establish model work. |
| Report | `messaging/delivery_report.rs` | Reads exact frozen event recipients and existing cursor; authentic snapshot progress is independent, legacy/remote facts stay unknown, namesake successors never inherit confirmation. |

The retained original logs support an incompatible-host failure, not a generic broken long connection or a demonstrated same-epoch backlog-loss defect. The bounded same-epoch controlled cases below reach actual Provider hook output and continuation, beyond socket readiness. No same-epoch durable-backlog loss was reproduced in this finite scope.

## Decisive path reached; evidence-assertion correction

`codex-r3.log` on candidate r2 reached actual waiting-child input and hook output for the ordinary initial message, queued active-turn message across daemon restart, and pre-retirement backlog after native kill/resume. The remaining failure was the inherited final managed-descriptor assertion comparing SQLite ownership to the original daemon PID even after the case intentionally restarted it. Actual descriptor evidence named only the new daemon. The assertion now compares `restarted.pid`; no production storage or delivery behavior changed. The case also records structured `MESSAGE_DELIVERY_EVIDENCE` and checks event-only report lookup does not alter caller cursor/status/context. Earlier Codex fixture nonempty-context assertion was corrected to accept the current instruction/context split's legitimate empty context. These remain test-only corrections, separately identified in the artifact receipt.


## Qoder native continuation versus terminal wake

Retained `qoder-final.log` reaches the queued body's actual Stop output (`decision:block`, `reason` contains the message), then fails while waiting for a second terminal-submitted delivery observation. `src/integrations/hooks/qoder.rs` owns this contract: Stop blocks completion to continue the current turn with pending context, whereas an empty Stop marks listening and requests a wake. The controlled Provider had ignored that output. Corrected the Qoder fixture to record the actual continuation and delivery directly, without a fabricated UserPromptSubmit. Shared evidence counts one activation (terminal submit or native Stop continuation) per delivered batch. This is a fixture correction; production Qoder behavior is unchanged. No Codex rerun is warranted by this Qoder-specific change.


## Final executed scope

Candidate r2 ordinary-send cases both pass: [Codex evidence](../../.artifacts/message-delivery/codex-evidence.json) / `codex-final.log` (16.61s), [Qoder evidence](../../.artifacts/message-delivery/qoder-evidence.json) / `qoder-final3.log` (14.24s). Each proves exact launched child receipt after a completed turn, pending backlog while active, same-generation daemon restart with the original child retained, eventual queued receipt, read-only lookup preserving caller cursor/status/context, and native kill/resume with the same Provider session consuming pre-retirement backlog. Qoder's resumed initial Stop uses the same centralized continuation recorder; its preceding failed run `qoder-final2.log` already contains the recovery body in actual Stop output and is retained as fixture evidence. Final source compile succeeded (`test-build-final3.log`). Existing focused native cases passed: seven acceptance/replay/receipt/report, one authenticated host progress and two compatibility cases. See [final result](result.md) for exact commands and limits.

Scope is native AgentX CLI/daemon/PTY with controlled Python Codex/Qoder substitutes on Linux. This proves supplied hook output/continuation, not paid/native model execution. No independent missed-hint fault injection or universal same-epoch liveness claim is made; the 30-second fallback remains source-established. Compatibility fences remain intact. The earlier incident before the correlated protocol-refusal slice is not fully reconstructed. Candidate production inputs are unchanged from packaging; the two test-only changes and final hashes are explicit in the [receipt](../../.artifacts/message-delivery/candidate-r2-receipt.json). [Cleanup evidence](../../.artifacts/message-delivery/cleanup-final.json) records all exact child/resumed/daemon PIDs absent after fixture cleanup. Original live hosts remain unmodified.
