---
schema: document
status: active
owner: messaging/runtime
links: [diagnosis.md, repair.task.md, ../../src/messaging/delivery_report.rs]
---

# Accepted messages and delivery recovery

`agentx send` commits a message before reporting acceptance. Its delivery observation distinguishes pending backlog, confirmed mailbox output, retired exact target and unknown historical/remote target. Confirmation means the mailbox cursor crossed that eligible message through the existing receipt boundary. It does not prove a model request started, work completed or a reply was sent. The new observation never confirms or drains mail.

`agentx send --delivery-status EVENT_ID --json` inspects the same accepted event without submitting again, changing caller activity or draining the caller inbox. Default send JSON includes `delivery`; human output includes the event and stage. Observation failure after acceptance remains explicit and does not turn the send into failure or authorize a duplicate submission. `--quiet` retains its output-suppression behavior.

For a registered PTY host, `last_poll` records a successful snapshot by its authenticated OS peer/incarnation. Model hooks cannot renew this signal. A recent snapshot proves only daemon communication, not a safe Provider gate or consumed backlog. Missing legacy progress remains unknown; a live process/socket is not proof of consumption. More than 65 seconds without snapshot progress is reported stale, with the recorded timestamp. This threshold is a diagnostic margin over the existing 30-second idle poll, not a lease, retry grant or message-expiration rule. Other Provider channels remain possible when no PTY host is registered.

The daemon serves PTY snapshots from its read queue. Fresh progress lives in memory under the exact process registration token. The existing heartbeat periodically checkpoints the last observed poll without advancing the mailbox cursor. After a daemon restart, reports use the persisted checkpoint if present and may look stale or unknown until the next authenticated poll. Replacing a process registration discards the old token's progress for the new host. See the [delivery and subscriber hot-path Task](../daemon-performance-audit/delivery-dispatch.task.md) for verification and rollout status.

## Supported explicit recovery

1. Save the accepted event ID and delivery report. Inspect `agentx term NAME`, `agentx daemon status --scope SCOPE`, and the exact participant/PID/Provider session facts. Do not resend the message or inject a terminal wake to claim delivery.
2. If a source-fixed compatible host still polls, wait for its active turn, approval or draft gate to clear through the Provider. Compatible daemon restart reconnects on a later one-shot service call while retaining the same database generation; pending mail remains durable.
3. If the host's loaded protocol is incompatible, changing the executable path or restarting only the daemon cannot change the loaded host. Coordinate an explicit native `agentx kill NAME --json` followed by `agentx resume NAME --headless` using a compatible immutable candidate and the same scope. Verify the exact original participant and Provider session before acting; a reused name is not authority over the original process. Native lifecycle tickets retain process/incarnation fences. Only perform this adoption when separately authorized for that live resource.
4. Check the resumed session identity, existing backlog body and actual receipt/cursor plus Provider continuation. Do not infer recovery from launch readiness. An old event's report remains retired when its original participant row was retired; it never credits a same-name successor merely because that successor has a larger cursor. Session lineage and the resumed consumer's receipt are separate recovery evidence.

The incident's original protocol2 processes remain preserved. This source change adds truthful caller-stage and host-progress observation plus regression evidence; it does not hot-update those images. Candidate acceptance and any later live adoption are recorded separately in the Task.
