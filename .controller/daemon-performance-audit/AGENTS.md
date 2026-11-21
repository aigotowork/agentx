# Daemon performance audit

Read the [design findings and priorities](audit.md), then the [repeatable SQL fixture](probe.py) and [recorded work](workload.json). Keep this audit distinct from the ongoing [isolation integration](../daemon-io-isolation.task.md). Source findings and in-memory query measurements do not establish live latency or authorize production schema operations. Preserve exact participant, message receipt, and generation semantics when implementing a candidate.

For the `dancingai` Codex resume incident, see [root cause, repair and rollout status](codex-resume.task.md) and the [isolated resume comparison](resume_probe.py).

For slow fresh `agentx l`, see the [launch audit](launch-audit.md) and [hot-path repair](launch-repair.task.md). The repair is separate from Codex preflight, caller wait policy, and live adoption.

The [implementation Task](implementation.task.md) owns current changes and final acceptance evidence.

[Foreground reads](foreground.task.md) owns short-lived Management projections and receipt observation.

[Delivery and subscriber hot paths](delivery-dispatch.task.md) owns the next native batch.

[Send and local release](send-release.task.md) owns the ordinary send follow-up and exact adoption evidence.

[Residual Busy and idle gates](residual-busy.task.md) owns queue attribution and the follow-up PTY workload.

[PTY completion and idle timers](pty-turn-completion.task.md) owns missed Stop reconciliation and staggered polls.
