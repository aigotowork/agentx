---
schema: task
status: cancelled
description: Diagnose and correct live daemon-busy admission failure affecting extension Agent discovery.
---

# Agent discovery 503 diagnosis

## Disposition

Controller cancels the unbound native implementation assignment at client closure: no professional native worker was admitted, no native correction was implemented or claimed, and later bounded endpoint reads recovered. The actual client failures were resolved by removing obsolete parent service discovery/optional permission gate and correcting standard browser-action dispatch. Earlier daemonBusy/host I/O observations remain valid historical symptoms with undetermined full cause; this is not a claim that native overload cannot recur. No unneeded daemon limit/retry redesign is required by the now-accepted client outcome. Any sustained recurrence belongs to a separately scoped native diagnosis from these observations, not a hidden completed native fix.

User cannot connect Chrome extension to default `http://192.168.2.178:38023`; controller independently reproduced descriptor `/api/v1/management` and `/space/` HTTP200 but `/api/v1/agents?generation=a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb&limit=100` HTTP503 `{"error":{"code":"Unavailable","message":"service unavailable: daemon is busy"}}`. AgentX CLI send also transiently returned busy. PID1149566 `.bin/agentx-web-context-r3 daemon serve --scope .../.agentx` remains live. This establishes service admission failure, not missing browser permission. Spoofed extension Origin from curl403 is not real privileged Chrome host evidence; do not confuse those.

Qualified coder assignment: native runtime/service diagnosis and smallest supported correction, no delegation. Read daemon/observability/service/management owning guides and systematic troubleshooting. Own native diagnosis record in this Task, necessary native affected source and focused tests/docs only after evidence supports cause. UI/client changes remain with client/Space owners; coordinate exact service error/recovery behavior to them. Controller @nube owns decision integration. No live restart/stop/reset, user-Agent mutation or runtime DB writes during diagnosis. Actual read-only OS/socket/runtime observation and finite HTTP checks allowed, avoid amplifying overload. Current source has MAX_CONNECTIONS32/QUEUE_CAPACITY16; Busy can originate admission or dispatch. Do not merely increase limits or add retries without distinguishing cause.

Use available native independent runtime inspection/observability, thread/socket/process evidence and bounded reproduction to distinguish sustained connection occupancy, queue saturation, long blocking dispatch, poll amplification, deadlock or environment. Recover causal model and retained invariants before code. Record exact evidence and selected correction, including whether running r3 lacks newer observation/source behavior. Prefer bounded disposable reproduction and meaningful regression. Cross-domain native changes owned here only if actually required; do not conflate client-error wording with native correctness.

If live service recovers naturally, still report evidence and durable client recovery implications; do not claim a native fix without cause. If deployment/restart is necessary, first produce verified candidate, consumer impact and precise selected-service plan; controller coordinates single live action and explicit authority, no surprise restarts. Existing service and all other previews are protected until then. Completion requires demonstrated cause/disposition and affected check, coherent handoff to client/independent acceptance and truthful actual running support. Notify @nube actionable diagnosis/gap/candidate, keep routine progress here.

## Initial read-only runtime evidence

Controller observed PID1149566 main thread in kernel state `D`, wait channel `folio_wait_bit_common`; most connection/worker threads wait on futex. `/proc/pressure/io` reported system-wide some avg10 51.89% / full avg10 31.83%, sustained avg60 63.16% /39.31%. This supports actual host filesystem/I/O waiting as a material diagnosis input, not proof of a complete daemon design cause. No process was stopped, no disk or runtime DB was mutated, and no limits were raised. AgentX launch and owner-message requests themselves returned busy, so no professional runtime binding has yet been established. Investigate filesystem/resource cause and admission recovery without attributing HTTP503 to extension permissions or treating PS1/browser issues as the cause.
