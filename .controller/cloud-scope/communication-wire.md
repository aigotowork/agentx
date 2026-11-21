# Scope communication contract

[Integration entry](AGENTS.md) owns the deployment interfaces and authenticated scope context. The native [wire DTOs](../../src/cloud/communication/types.rs) and [HTTP bindings](../../src/cloud/communication.rs) own precise shapes and emit the scope OpenAPI. Scope paths below are relative to the configured `/scopes/{id}/api/v1` base. Every request uses the selected scope's Bearer. Every write includes the descriptor's `generation`; host mutations also identify the client installation. The installation ID detects name collisions and is ordinary host state, not a device credential. Body/query errors use `{code,message}`.

| Method and route | Input beyond path | Output |
| --- | --- | --- |
| GET /descriptor | — | `{scope_id,generation,protocol_version}`, version 1 |
| GET /hosts | optional `after,limit` | `{items:HostView[],next}` |
| GET /agents | optional `after,limit,host` | `{items:AgentView[],next}` |
| PUT /hosts/{host} | `{generation,installation_id}` | HostView |
| PUT /agents/{name} | `{generation,installation_id,host,incarnation}` | AgentView |
| POST /agents/{name}/retire | same as registration | exact retired AgentView |
| POST /messages | `{generation,installation_id,request_id,from:AgentView,to,body,intent,reply_to?,thread?}` | MessageView |
| GET /operations/{request_id} | — | `{request_id,status,message:MessageView}` |
| POST /inbox/claim | `{generation,installation_id,agent:AgentView,claim_id,limit?,wait_ms?}` | `{claim_id,items:MessageView[],next:null}` |
| POST /inbox/confirm | `{generation,installation_id,agent:AgentView,claim_id,message_ids}` | `{confirmed:string[]}` |
| POST /inbox/recover | `{generation,installation_id,agent:AgentView,claim_id,release?:boolean}` | `{claim_id,items:MessageView[],next:null}` |

HostView is `{name,last_seen,online}`; AgentView is `{name,host,incarnation}`. MessageView is `{id,from:AgentView,to:AgentView,body,intent,reply_to,thread,created_at}`. Required nullable response fields are emitted as null. Intent is `request|inform|ack`. Message IDs are positive signed-64-bit decimal strings; timestamps are JSON integer Unix seconds. Directory `next` is the last returned name when another page exists. Directory pages allow 1–200 items and default to 50; inbox batches allow 1–100 items, default to 20, and stop at 1 MiB of total message bodies. Each message body is limited to 64 KiB. Claim waits default to zero and are capped at 20 seconds, with capacity reserved for sends and credential changes. The waiter subscribes before checking its durable inbox. Committed sends, releases, retirements and token rotation wake requests; idle waits hold neither a database transaction nor a blocking worker. Send accepts `@name` and `@host/name`; the qualified form requires the exact host. Reply IDs refer to prior delivery between the same exact instances in opposite directions.

| State and transition | Durable rule |
| --- | --- |
| Host enrollment / heartbeat | First installation reserves the host name. Only the same installation can renew it. Last contact is online for 60 seconds; losing the lease never frees names or queued messages. |
| Agent registration / retirement | One active registration per short name across the scope. Same exact registration can retry while active. Retirement permanently fences its incarnation; a later registration needs a new incarnation. Host online state is evidence about host contact; it does not prove a Provider is running. |
| Send / operation lookup | Validate the exact live sender, canonicalize and fingerprint the submitted command, and freeze the registered target and body in one transaction. A repeat of an accepted identical request ID returns the original result before resolving current names; reusing the ID with changed input conflicts. An offline registered target can receive queued messages. A retired target's unconfirmed messages report `undeliverable`. |
| Claim / confirm | An exact Agent can claim a bounded batch in ID order. Client generates a claim ID and may repeat that ID to inspect the remaining items. A different claim ID returns HTTP 409 with code `claim_conflict` while an outstanding batch exists. An empty claim reserves nothing. Confirm only after Provider evidence; confirms are atomic for the supplied IDs and idempotent for the same claim. |
| Recover | Inspect the remaining unconfirmed batch after a lost response. Confirm again if the Provider accepted it. Explicit release returns the remaining items to the queue only when the local host knows it is safe to redeliver. A released claim ID remains closed. The service does not expire claims into automatic redelivery. |
| Rotation | Reauthorization applies to new calls and outstanding waits. Committed data and claims survive token changes. A mutation already in progress when the token rotates can have committed; inspect its exact ID using the new bearer. |

`communication.rs` owns Axum extraction, authorization context use, HTTP status mapping and bounded waits. `communication/types.rs` owns public DTOs and shared wire limits. `communication/store.rs` owns SQLite initialization, generation fencing and directory/send transactions; `store/inbox.rs` owns claim, confirmation and recovery inside those transactions. Admin uses generation-fenced read-only counts and bounded directory pages; it receives only directory metadata. No local PID, environment, filesystem path, or Provider state enters these DTOs.

The cloud store is a separate identity domain because an HTTPS scope member has no native daemon peer, PID, or Provider session that the server can verify. The scope Bearer admits membership; the scope generation, persisted host installation, and exact Agent incarnation bind names and freeze routing and mailbox ownership across hosts. Each local daemon remains the authority for its processes, sessions and attachments. Cloud and local sends share [native message text validation](../../src/messaging/routing.rs); the additional 64 KiB cloud bound limits transport and mailbox cost.

Verification focuses on request replay versus changed payload, offline mailbox durability, claim ownership and recovery, retirement versus replacement, and OpenAPI correspondence. Authentication, credential rotation across real HTTP waits, and normal Provider launch/send/reply are integrated separately by the [acceptance task](acceptance.task.md).
