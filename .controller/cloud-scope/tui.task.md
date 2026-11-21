---
schema: task
status: completed
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"lubo","incarnation_bits":4745316805039819404}}]
---
Read [the integration contract](AGENTS.md).
Implement and test deployment admin TUI in src/cloud/tui.rs. Expose run(url:&str)->anyhow::Result<()>. Leave existing dirty TUI files alone.

The deployment operator starts at a compact scope table with aggregate counts and opens the exact selected scope to inspect hosts or Agents. Inspection remains adjacent on wide terminals and uses the same workspace on narrow terminals. A modal pins the create or rotation target; rotation uses the observed credential version. The operator explicitly reveals and saves a generated communication token before submitting. A missing result retains the identical token, request ID and request payload for an explicit retry; success leaves the token recoverable from the receipt until dismissed. The TUI owns interaction and rendering only; it uses the shared cloud HTTP transport and the [admin contract](AGENTS.md).

Focused checks: observe exact identity and host/Agent pages across navigation; exercise create and rotate through their visible confirmation and verify the HTTP effect; simulate a lost response and assert identical replay; inspect narrow rendering and ensure tokens are absent before explicit reveal. Full deployment acceptance remains with the [acceptance Task](acceptance.task.md).

Implemented in [the deployment TUI](../../src/cloud/tui.rs). Keys: `n` creates, `Enter` inspects the selected scope, `Tab` switches host and Agent lists, `Enter` on a host filters Agents by exact host, and `r` rotates the selected scope's token. In a credential dialog, `Tab` reveals or hides the token; `Enter` submits only after reveal and replays the same request after an unknown result. An operator saves the generated token before submission; the dialog retains replay data until dismissed.

Linux verification: [two focused Rust tests](../../.tmp/cloud-scope/lubo-tui-xFkK6a/targeted-tests.log) passed for exact replay through a loopback HTTP fixture, masking, and selected scope/version through a refresh. [Fourteen actual terminal checkpoints](../../.tmp/cloud-scope/lubo-tui-xFkK6a/pty-result.json) passed through the production TUI, shared transport, real deployment catalog/router and isolated tmux. The run checked creation, host-filtered and scope-wide Agent inventory, resizing from 102×24 to 42×18, v1→v2 rotation, old-token rejection, new-token authorization and original TTY attribute restoration. The slice executable omitted unrelated native dispatch and Web assets; the [acceptance Task](acceptance.task.md) owns combined candidate checks. The earlier workspace build was blocked by in-flight native client changes; no installed binary or live service was changed.
