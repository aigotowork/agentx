---
schema: task
status: completed
agents: [{"scope":"/home/long/works/longwork/agentx/.agentx","generation":"a6195a5b-1dd4-4a6d-87d4-1f18f1b664fb","participant":{"canonical_name":"zimu","incarnation_bits":4745316805023570206}}]
---
Read [the integration contract](AGENTS.md).
Implement communication service and scope OpenAPI in src/cloud/communication.rs and submodules per integration contract. Expose document() -> serde_json::Value. Coordinate wire with client and SDK immediately.

The [wire contract and state model](communication-wire.md) resolves launch snapshot gaps: the approved plan requires persistent name reservations while offline, explicit claim and confirmation, and generation fences. Native owner uses installation IDs and exact registered senders. Parent deployment owner supplies credential version rechecks; the communication handler rechecks on either side of long waits. Directory, send and inbox state share one SQLite transaction boundary.

Implemented the [scope router and OpenAPI emitter](../../src/cloud/communication.rs), [shared native DTOs](../../src/cloud/communication/types.rs), and [generation-fenced SQLite operations](../../src/cloud/communication/store.rs). Admin now consumes bounded counts and paginated metadata through the communication boundary. Native clients share the wire DTOs; the emitted scope document matches the SDK input exactly.

Verification: all six scope source tests passed in the isolated Rust harness using the repository's built dependencies. They cover offline registration and cross-host reply, idempotent sends, claim recovery and confirm atomicity, retirement and replacement fencing, rotation/generation transaction checks, response bounds and OpenAPI schema. The exact emitted spec matched `packages/agentx-client/cloud-scope.openapi.json` byte for byte. Integrated native HTTP, real Provider host delivery, and CLI/TUI/Web remain the [independent acceptance task](acceptance.task.md); no running service was changed.

Follow-up (native text safety): the authoritative send transaction now calls the same validator as native messaging and retains the cloud 64 KiB ceiling. The targeted regression passed with the exact native validator source in the isolated scope harness: ESC and C1 control sequences and oversized text produce neither a receipt nor mailbox entry; valid multiline Unicode can still be sent using the same request ID. OpenAPI output remains byte-for-byte identical to the SDK input. The [identity rationale](communication-wire.md) records why the scope store cannot reuse local PID/session identity. Integrated product evidence belongs to the acceptance task.
