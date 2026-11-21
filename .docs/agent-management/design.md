---
schema: document
description: Native contracts and daemon composition for the AgentX management HTTP API and Web console.
status: active
owner: agent-management
last_updated: 2026-11-15
links: [../proposals/agent-management-http.md, ../../.controller/agent-management/delivery.md]
---

# Agent management API

## Model and ownership

One selected AgentX data scope owns the daemon, native Agent state and management operation receipts. An Agent reference is a canonical name plus its exact execution incarnation, or its exact stopped-history event. Every mutating request and detail read carries the current database generation. A name reused by a successor cannot receive work intended for an older reference. Stopped history remains readable and resumable through its original snapshot. Provider support still follows existing runtime capabilities. Local-origin projections follow native row normalization: both NULL and an empty `origin_device_id` represent local rows; nonempty remote origins are excluded before pagination.

`src/management/contract.rs` owns the native HTTP/read/operation types. `store.rs` owns bounded read projections, command admission and durable operation receipts using daemon-owned KV storage. `worker.rs` composes existing runtime/messaging/transcript functions in a separate same-binary host process. `service.rs` owns the captured native consumer capability and asynchronous worker supervision; `service/resources.rs` owns typed resource/office/attachment supply and `service/inspection.rs` owns bounded Inspector binding. `web.rs` owns HTTP parsing, limits and actual wire projection. No browser payload deserializes the aggregate `service::Request`.

The HTTP server runs on a daemon-owned thread/runtime. Native calls use the existing authenticated local IPC and single SQLite dispatcher; external host work never executes on that dispatcher. The worker receives a frozen invocation and private claim ticket over stdin, enters that invocation's environment/cwd in its own process and claims the operation with its real OS identity. The successful claim returns the accepted command. It never opens managed storage. This isolates legacy runtime code that requires the originating process environment without a new execution platform or daemon-global `set_current_dir`/environment mutation.

At most four workers run per daemon even when HTTP configuration changes. Each worker has a 120-second execution budget; expiry stops the operation worker and records an unknown outcome without claiming to stop any already-created Agent. The selected scope's environment-file resolver runs in the isolated host using the operation working directory and explicit scope. It preserves the daemon's inherited base environment as configuration input, strips Agent/terminal identity, and never treats that base as an authenticated Agent identity.

## Configuration and lifetime

Scope `config.toml` selects `[management] enabled = true`, `host = "127.0.0.1"`, `port = 0` and `project = "/absolute/project"`. Port 0 allocates an available non-privileged port. Explicit non-loopback binding enables direct LAN management for clients reaching the listener; use a trusted network. No new account or permission service. The [shared browser access policy](../api-panorama/openapi/README.md#browser-access-policy) permits cross-origin requests, preflight and iframe embedding while retaining native authority/identity/validation. The daemon writes `.daemon/management.json` with its instance, URL and project after bind and removes only its own descriptor at shutdown. `agentx daemon status` reports this endpoint. `agentx daemon web [--scope PATH] [--project PATH] [--host IP] [--port PORT]` configures/enables and acquires the daemon, returning the endpoint; a running daemon must not be silently replaced.

The journal binds the original HTTP command, including omitted/default directory, before host resolution. Each new launch freezes its selected project/directory in the worker invocation; replay never revalidates the current filesystem to find an existing receipt. Directory or environment preparation failure after admission yields a retained operation outcome rather than a replacement request.

HTTP loss or page close does not cancel a started operation or its Agent. The operation journal binds a caller-provided UUID to the exact command and generation. Replay returns the same receipt even when the worker pool is full; a different command with that ID conflicts. Pending/running records survive restart; abandoned work becomes outcome-unknown and is never automatically re-executed. A successful journal claim is the only permission to execute once and returns the canonical accepted command, rather than trusting a second command copy from worker input. An unclaimed grant expires after 30 seconds; it cannot be claimed after that deadline. Completed receipts survive an ordinary daemon restart, and reset rejects the old generation. Receipts remain in the selected dataset without automatic eviction so old IDs cannot silently become fresh operations.

## HTTP contract

All responses are ordinary JSON; IDs, event cursors and incarnation values are decimal/opaque strings to preserve JavaScript precision. Requests reject unknown fields, malformed/duplicate JSON keys and out-of-range sizes. Reads return no host secrets, launch environment or claim tickets. Errors use `{error: {code, message}}`. Native declarations are canonical; generated TypeScript derives from them.

| Method/path | Input | Output |
| --- | --- | --- |
| `GET /api/v1/management` | none | Descriptor: generation, scope/project, tools/profiles and limits |
| `GET /api/v1/agents` | generation, optional after/limit | AgentPage with live and bounded stopped references |
| `POST /api/v1/agents` | SubmitLaunch: request_id, generation, launch | durable Operation receipt; HTTP 202 |
| `GET /api/v1/agents/{name}` | generation, incarnation or stopped_event | exact AgentView |
| `GET /api/v1/agents/{name}/messages` | generation, exact ref, optional after/limit | MessagePage |
| `POST /api/v1/agents/{name}/messages` | SubmitMessage: request_id, generation, agent, message | durable Operation receipt |
| `POST /api/v1/agents/{name}/stop` | SubmitTarget: request_id, generation, agent | durable Operation receipt |
| `POST /api/v1/agents/{name}/resume` | SubmitTarget | durable Operation receipt |
| `POST /api/v1/agents/{name}/transcript` | SubmitTarget | durable Operation; result contains bounded transcript |
| `GET /api/v1/operations/{request_id}` | generation | current Operation with typed result or explicit uncertainty |

Transcript uses a read job so file I/O runs outside daemon dispatch and a disconnect can retrieve the same snapshot result. Native command variants are Launch, Send, Stop, Resume and Transcript. Launch uses one Agent, saved profile or supported tool, explicit directory/default project and optional initial prompt; host mode is background. No raw argv, arbitrary PID, shell command, environment-map or generic native proxy endpoint. Sending uses existing external sender `bigboss`, matching TUI semantics. Message receipt means admitted, not delivered or answered.

Polling uses bounded, cancellable reads every two seconds. Messages and stopped records have independent event cursors. Message projections use the native creation timestamp as their lower boundary and the exact stopped event as their upper boundary, preventing a namesake successor from changing an earlier execution's communication history. Provider model transcripts retain their native session meaning; a resumed session can include earlier model exchanges. No PTY bytes, token stream, WebSocket or framework-level query platform is needed. Browser drafts and selection survive polling; an uncertain write retains its request ID and retrieves its outcome before any new attempt.

## Scenario and validation

The core path is launch → select Agent → send work → see acceptance and reply → read model transcript → stop → read stopped history → resume. Operation status, Agent execution status and task completion are separate. Empty/failed/missing transcript and unsupported resume have explicit UI states. Mobile uses the same selection/detail path with local panels.

Operator retirement and Provider callbacks can overlap. A launched hook may bind or switch sessions only while its original process binding still resolves to a live, non-killed row. Missing/deleted bindings cannot enter unbound identity creation, and an ordinary status callback cannot overwrite the operator's killed marker. These guards belong to native participant binding/status admission, so CLI and HTTP stop share the same behavior. Genuine unlaunched session adoption and current launched session switches retain their existing policies; resume reserves a fresh execution before its hooks arrive.

Independent acceptance uses a real HTTP listener, same-binary worker and native hooks with an isolated local Provider fixture. It must also exercise duplicate writes, ID conflict, stale incarnation, changed generation, disconnect/reload and cleanup. Packaging embeds fresh React resources. Tests and delivery records identify fixture, actual Provider and platform scope separately.

## Visual office follow-up assessment

The [source-verified office supply assessment](office-supply-assessment.md) dispositions adopted workspace/Role projections, known-office/catalog limits and the native pure endpoint inspection needed for read-only routing. It is an assessment of a separate consumer follow-up, not implemented support or a reopening of the accepted management Delivery.

The user subsequently authorized holistic implementation of A/B/C together; [unified office design](offices.md) and [Delivery](../../.controller/office-management/delivery.md) own the selected extension and actual execution state.
