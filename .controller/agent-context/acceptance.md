---
schema: document
status: accepted-bounded-local
owner: independent-verifier
last_updated: 2026-11-14
related: [delivery.md, ../../.docs/proposals/agent-context.md, ../../tests/agent_context/README.md]
---

# Independent context acceptance

## Verdict

**Accepted for the bounded Linux local scope below.** Independent composer, plugin, candidate CLI/config/launch, ordinary Codex/Claude hook and real Codex manual/automatic compaction checks passed. No substantive correctness failure remains in these executed cases. This verdict establishes native Codex request construction against a deterministic local model protocol, not hosted-model semantic compliance or universal Provider support.

Candidate: `/tmp/agentx-context-candidate`, version `0.7.31`, independently verified SHA-256 `245ad1f22467ac3978002b4c309231b628aa0ef1b23f9c1f9f85a2da1a4784bc`. The user explicitly handed off this final artifact after home/profile preservation changes. Checkout HEAD recorded at closure: `b118286b61833dae6821e35f8299360a0670894f`; the candidate is identified by its digest, not an assumption that concurrent uncommitted source equals HEAD. Source-only receipts separately retain their source hashes.

[Delivery](delivery.md) owns implementation/build provenance. Its final package used the documented local non-LTO release overrides; no default-profile performance/distribution acceptance is implied. The reported strict all-target Clippy failure in existing/concurrent files is not a passed check. The verifier ran no main-project Cargo build or Clippy command.

## Executed evidence

| Surface | Result | Evidence and supported claim |
| --- | --- | --- |
| Independent composer crate | 3 tests passed | [Receipt/source hashes](../../tests/agent_context/evidence/source-ky08b5h7/summary.json), [log](../../tests/agent_context/evidence/source-ky08b5h7/composer.log): default/disabled selection, structural rejection, ordered literal sources, empty blocks, current reason/facts, repeated file reads, missing/invalid UTF-8/unknown generator failures, exact multibyte and aggregate separator bounds, no successful partial output |
| Pi, OMP, OpenCode v1 source plugins | All three adapter scenarios passed | [Log](../../tests/agent_context/evidence/source-ky08b5h7/plugins.log): fresh repeated compact context, empty/failed refresh clearing, foreign-session suppression and no compact status/mail RPC or synthesized user message; OpenCode next ordinary request uses new context, clears old context after RPC failure and recovers after a successful retry |
| Candidate CLI/launch | 20 cases passed | [Results](../../tests/agent_context/evidence/candidate-launch-corrected.log), [command receipts](../../tests/agent_context/evidence/candidate-launch-receipts/summary.json): legacy notes/defaults, disabled/replacement context, empty/JSON env overrides, ordering, caller-relative files, invalid/missing/unknown/oversized source errors and native instruction precedence |
| Candidate config key | 5 cases passed | [Results](../../tests/agent_context/evidence/candidate-config.log), [receipts](../../tests/agent_context/evidence/candidate-config-receipts/summary.json): JSON key writes a TOML source array, readback, duplicate-ID rejection without file mutation, explicit empty array, unset |
| Ordinary candidate hooks H01/H02 | 8 checks passed | [Results](../../tests/agent_context/evidence/candidate-hooks.log), [receipts](../../tests/agent_context/evidence/candidate-hook-receipts/summary.json): two current-file compact refreshes each for Codex/Claude despite `name_announced=1`, preserving active status/context/detail and session; Codex missing/empty/wrong/retired session callbacks produce no context or observed position/binding/state change |
| Real Codex manual compaction | Passed | [Receipt](../../tests/agent_context/evidence/native-manual-receipts/summary.json), [observations](../../tests/agent_context/evidence/native-manual-receipts/observations.json), [wire trace](../../tests/agent_context/evidence/native-manual-receipts/trace.jsonl): real `thread/compact/start`, one native compaction item, fresh postcompact developer context, retained native developer instructions, exactly two intended user messages |
| Real Codex automatic compaction | Passed | [Receipt](../../tests/agent_context/evidence/native-auto-receipts/summary.json), [observations](../../tests/agent_context/evidence/native-auto-receipts/observations.json), [wire trace](../../tests/agent_context/evidence/native-auto-receipts/trace.jsonl): token-threshold automatic compaction occurs within the second user turn, shares its native turn ID, then sends refreshed developer context while retaining native instructions; exactly two intended user messages |

The composer uses its own [manifest](../../tests/agent_context/Cargo.toml) and [lockfile](../../tests/agent_context/Cargo.lock), with external target `/tmp/agentx-context-verifier-target-qpw89r25`. It ran offline against the imported production composer; it did not rebuild the application. Passed source checks were not repeated when only the separate native-launch helper changed.

## Native execution and interpretation boundary

The real executable is Codex `0.167.0`, native SHA-256 `cdcb45aaa01d0f14e298985e09465aebccd47f64ecd298aea44814af7b62bbda`. [Installed protocol discovery](../../tests/agent_context/evidence/native-protocol.json) established the manual compact method. [Runner](../../tests/agent_context/native_compact.py) creates a fresh provider home, project, AgentX scope and loopback HTTP model fixture with `requires_openai_auth=false`. It inherits no credentials or user configuration.

For ordinary process binding, the owned [entry shim](../../tests/agent_context/native_entry.py) converts the final native `exec` token to `app-server --stdio`, passing through the real candidate-generated hook/developer overrides. This uses the real Codex executable, lifecycle, compaction, hook execution and outbound model-request construction. It intentionally does not enter AgentX's managed shared app-server path. The deterministic model server returns fixed Responses-protocol output and controlled token usage; it does not test a hosted model's interpretation, instruction obedience, inference quality or authentication.

In both accepted native runs, the next model request contains `NATIVE_AFTER_COMPACT` in a **developer-role** item and retains `NATIVE_DEVELOPER_KEEP`. Manual compaction has a separate native maintenance turn; automatic compaction shares the second intended user turn ID. Each trace contains one compaction item, three model requests including the compaction request, and exactly two user-message items. The native traces establish no synthesized extra user message. Active AgentX status preservation is separately established by the ordinary hook before/after snapshots; the native runner does not claim an additional daemon-status observation at every internal compact instant.

The [official OpenAI hooks contract](https://developers.openai.com/codex/hooks/) was fetched during preparation. Candidate [hook registration](../../src/integrations/hooks/codex.rs) sets `additionalContextLimit=0` for bounded composer output; the native wire evidence above, rather than that declaration alone, supports postcompact developer-context delivery.

## Correctness findings and dispositions

| ID | Disposition |
| --- | --- |
| F01: ordinary compact callback without exact session | Corrected by implementer and verified in the candidate. Missing, empty, wrong and retired session IDs return no context and preserve recorded binding, cwd/transcript and active state. Positive controls refresh twice. [Vectors](../../tests/agent_context/compact_sessions.json), [H01/H02 receipt](../../tests/agent_context/evidence/candidate-hook-receipts/summary.json). |
| F02: native home/profile developer instructions overwritten by generated startup override | Bounded correction verified. Home instructions, default native profile and explicit profile forms (`--profile NAME`, `--profile=NAME`, `-p NAME`) survive generated argv; explicit developer argv wins over home/profile, including attached `-cdeveloper_instructions=`. Native wire tests also show home developer instructions after manual/automatic compaction. Project/system config discovery remains intentionally outside the correction. |
| OpenCode failed refresh could retain old context | Source scenario passed: seed nonempty cache, fail RPC, verify no stale compact or next-request injection, then recover on successful refresh. This is mocked-RPC plugin evidence, not native OpenCode acceptance. |

No verifier source edits were made outside `tests/agent_context/` and this acceptance file. No stylistic findings were used as acceptance blockers.

## Explicit unexecuted boundaries

- Managed shared Codex sibling/root attachment isolation, concurrent shared clients and wrong-sibling compact delivery are **unexecuted**. Ordinary wrong-session rejection does not establish shared-root fencing.
- OpenCode v2, live OpenCode/Pi/OMP hosts and delayed in-flight RPC completion after a session switch are **unexecuted**. Source mocks establish only their exercised event paths.
- Claude compact events were executed through actual candidate hooks with a finite provider substitute. A real Claude compact/model request, Qoder native recovery and other providers remain **unexecuted**.
- Pending-mail nonacknowledgment across native compact, failure during hook stdout flush, and every native internal active-state instant were not independently exercised. The source plugin cases show no compact status/mail RPC; that is not a replacement for mailbox integration evidence.
- Hosted-model semantic compliance, credentials/account compatibility, project/system Codex config discovery, other platforms, full-project lint/test health, release performance and installed/live-service deployment are outside this acceptance.

These limits are retained support boundaries, not requests for another validation campaign. The user explicitly selected closure with this bounded evidence.

## Resource cleanup and retained attempts

All **20** candidate-launch fixture daemon peers and **two** ordinary-hook fixture peers have successful exact-incarnation stop receipts. Each accepted native run stopped its recorded AgentX daemon peer, closed/waited for its owned native process and shut down its local HTTP server. Config-only checks created no daemon. The earlier native harness attempt also stopped its fixture daemon. Plugin checks mocked listeners/process spawning and created no live provider resource. No user runtime, installed binary, user credential, project daemon or LAN demonstration was modified. Temporary directories/build output remain as retained artifacts; they are not running resources.

Two harness corrections are retained transparently. The [first launch attempt](../../tests/agent_context/evidence/candidate-launch.log) used obsolete `codex` shorthand and is entirely excluded, including apparent negative passes; corrected execution uses `launch --tool codex` and requires a context-specific failure diagnostic. The [first native manual attempt](../../tests/agent_context/evidence/native-manual.log) observed a real compact but matched the maintenance turn's completion too early; it is excluded from acceptance. The corrected runner correlates completion to the requested second turn ID, and the accepted manual/auto receipts above contain the decisive wire evidence.

Paseo communication was used initially. After the user clarified routing, no further messages were sent to the historical controller. This acceptance and the completion response are the final handoff; Mika remains the implementation owner.
