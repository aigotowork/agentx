---
schema: task
status: completed
description: Implement the exact-target recover CLI and one safe managed Codex continuation per live model-error episode.
links: [../delivery.delivery.md, ../../../.docs/proposals/agent-recover.md, ../../../.docs/model/model-request-errors.md, ../../../src/inspection/recovery.rs]
---

Read the [accepted design](../../../.docs/proposals/agent-recover.md), [Delivery](../delivery.delivery.md), CLI/Agents module guidance and existing model-error, label-selection and PTY delivery contracts before editing.

Implement one cohesive path:

- Add `recover @name...`, `recover --all`, and omitted-target interactive multiselect. Reuse exact local name resolution, existing labels (AND predicates), workspace default, `--all` scope and searchable `select_multiple`. Candidates require current exact incarnation, live managed Codex PTY, matching session, failed unresolved model request, no approval/exit and a safely ended turn.
- Record the latest accepted model failure and a subsequent same-session Stop in the participant recovery store. Keep one durable attempt per unresolved episode. A queued attempt may expire safely before the exact registered PTY claims it; once claimed, it must never be reissued blindly.
- Extend the existing delivery snapshot/control with a recovery ticket. In the Codex PTY loop, apply the native prompt gate before typing and reauthorize the same ticket and prompt ownership before pressing Enter. Use a fixed continuation prompt separately from the ordinary mailbox and its cursor.
- Emit per-Agent accepted, already-requested, skipped and failed results; a later model-request success observation alone confirms recovery. Continue remaining batch items after one failure.
- Add focused native admission/delivery cases and CLI parser/selection cases before or with the implementation. Update command help, root usage, and the model/feature entry points. Run the smallest focused source checks after the coherent change; packaging and installed/live adoption remain separate.

Preserve live daemons, Agent hosts and unrelated files. Complete after the focused source and isolated CLI/PTY checks; record any unsupported host or effect uncertainty in this Task and Delivery.

Implementation decision: a marked mailbox message would either wait behind unrelated unread mail or cause the ordinary wake to drain that mail. Use a dedicated, expiring native continuation ticket on the existing registered host loop instead. The stop observation lives separately from sticky model status; a seconds-resolution last-stop clock cannot order a model failure and its turn end. Existing message, Relay and Management contracts are unaffected.

The first controlled Codex PTY run registered the live child and reported a model failure and Stop, but discovery excluded it because recovery looked for `AGENTX_PTY_MODE=1`. Both [foreground and background launch](../../../src/runtime/launcher.rs#L718) use the same tool-specific environment; that variable opts Claude Stop hooks into PTY-owned input delivery. Existing launcher tests explicitly require its absence for background Codex. Codex recovery now validates its native process binding, and discovery/admission additionally require the exact live tracked child and registered PTY host. No launcher environment changes are needed.

The corrected candidate passed the isolated headless Codex recovery scenario without supplying the Claude-specific variable. See the [verified outcome and exact artifact](../delivery.delivery.md#verified-linux-outcome).
